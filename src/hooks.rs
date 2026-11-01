//! Sections drawn from a shell command. The line shows a hook's cached output
//! and never waits on it: a stale or missing cache starts a detached refresh
//! that the next render picks up.

use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime},
};

use jiff::SignedDuration;

use crate::host;

/// The hidden argument that runs a hook in a detached child.
pub const HOOK: &str = "--hook";

/// What a hook sees of the session: the JSON Claude Code sent, and where it runs.
pub struct Session<'a> {
    pub id: &'a str,
    pub input: &'a [u8],
    pub dir: &'a str,
}

pub struct Hook<'a> {
    pub name: &'a str,
    pub run: &'a str,
    pub refresh: SignedDuration,
    pub timeout: SignedDuration,
}

/// The hook's last output, refreshing it in the background once it is older than `refresh`.
pub fn text(hook: &Hook<'_>, session: &Session<'_>) -> Option<String> {
    let cache = cache_dir().join(format!("{}.{}", hook.name, key(session.id)));
    let cached = fs::read_to_string(&cache).ok();
    if age(&cache).is_none_or(|age| age >= hook.refresh) && !pending(&cache, hook.timeout) {
        let _ = spawn(hook, session, &cache);
    }
    cached.filter(|text| !text.is_empty())
}

fn cache_dir() -> PathBuf {
    host::var("XDG_CACHE_HOME")
        .map_or_else(|| host::home().join(".cache"), PathBuf::from)
        .join("linea")
        .join("hooks")
}

fn key(session: &str) -> String {
    let key: String = session
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if key.is_empty() { "none".into() } else { key }
}

fn age(path: &Path) -> Option<SignedDuration> {
    let modified = fs::metadata(path).and_then(|meta| meta.modified()).ok()?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default();
    SignedDuration::try_from(age).ok()
}

fn pending_path(cache: &Path) -> PathBuf {
    beside(cache, ".pending")
}

fn beside(cache: &Path, suffix: &str) -> PathBuf {
    let mut path = cache.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}

/// A refresh already running, unless it outlived its timeout.
fn pending(cache: &Path, timeout: SignedDuration) -> bool {
    age(&pending_path(cache)).is_some_and(|age| age < timeout + SignedDuration::from_secs(1))
}

fn spawn(hook: &Hook<'_>, session: &Session<'_>, cache: &Path) -> std::io::Result<()> {
    fs::create_dir_all(cache_dir())?;
    fs::write(pending_path(cache), [])?;
    let timeout = u64::try_from(hook.timeout.as_millis()).unwrap_or(0);
    let mut child = Command::new(std::env::current_exe()?)
        .arg(HOOK)
        .arg(cache)
        .arg(timeout.to_string())
        .arg(session.dir)
        .arg(hook.name)
        .arg(hook.run)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    child
        .stdin
        .take()
        .map_or(Ok(()), |mut stdin| stdin.write_all(session.input))
}

/// The detached child: runs the hook within its timeout and caches the first line it prints.
pub fn refresh(mut args: impl Iterator<Item = OsString>) {
    let (Some(cache), Some(timeout), Some(dir), Some(name), Some(run)) = (
        args.next(),
        args.next(),
        args.next(),
        args.next(),
        args.next(),
    ) else {
        return;
    };
    let cache = PathBuf::from(cache);
    let timeout = Duration::from_millis(timeout.to_string_lossy().parse().unwrap_or(0));
    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);

    let text = execute(&run, &dir, &name, &input, timeout).unwrap_or_default();
    let partial = beside(&cache, ".partial");
    if fs::write(&partial, text).is_ok() {
        let _ = fs::rename(&partial, &cache);
    }
    let _ = fs::remove_file(pending_path(&cache));
}

fn execute(
    run: &OsString,
    dir: &OsString,
    name: &OsString,
    input: &[u8],
    timeout: Duration,
) -> Option<String> {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(run)
        .env("LINEA_SECTION", name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if Path::new(dir).is_dir() {
        command.current_dir(dir);
    }
    let mut child = command.spawn().ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input);
    }
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        out
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().ok()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let out = reader.join().ok()?;
    status.success().then(|| {
        String::from_utf8_lossy(&out)
            .lines()
            .next()
            .unwrap_or_default()
            .trim_end()
            .to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_key_is_a_safe_file_name() {
        assert_eq!(key("a1b2-c3"), "a1b2-c3");
        assert_eq!(key("../x"), "___x");
        assert_eq!(key(""), "none");
    }

    #[test]
    fn a_hook_prints_its_first_line_within_its_timeout() {
        let run = |cmd: &str, timeout| {
            execute(
                &cmd.into(),
                &"/".into(),
                &"t".into(),
                b"{\"a\":1}",
                Duration::from_millis(timeout),
            )
        };
        assert_eq!(run("printf 'hi  \\nthere'", 2000).as_deref(), Some("hi"));
        assert_eq!(run("cat", 2000).as_deref(), Some("{\"a\":1}"));
        assert_eq!(run("echo $LINEA_SECTION", 2000).as_deref(), Some("t"));
        assert_eq!(run("exit 1", 2000), None);
        assert_eq!(run("sleep 2", 50), None);
    }

    #[test]
    fn the_pending_marker_sits_beside_the_cache() {
        assert_eq!(
            pending_path(Path::new("/c/weather.s1")),
            Path::new("/c/weather.s1.pending")
        );
    }
}
