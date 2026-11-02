//! Sections drawn from a shell command. The line shows a hook's cached output
//! and never waits on it: a stale or missing cache starts a detached refresh
//! that the next render picks up.

use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime},
};

use jiff::SignedDuration;
use serde::{Deserialize, Serialize};

use crate::{
    child,
    config::{Config, Hook, SectionName, Source},
    host,
    session::{Session, SessionId},
};

pub const SUBCOMMAND: &str = "refresh";

/// What a detached child needs to run one hook.
#[derive(Debug, Serialize, Deserialize)]
pub struct Refresh {
    pub cache: PathBuf,
    pub section: String,
    pub command: String,
    pub dir: String,
    pub timeout: Duration,
    /// The status JSON exactly as Claude Code sent it.
    pub input: String,
}

/// Each hook section's last output, by section name.
#[derive(Debug, Clone, Default)]
pub struct HookOutputs(BTreeMap<SectionName, String>);

impl HookOutputs {
    /// Reads every hook section's cached output, starting refreshes for stale ones.
    pub fn read(config: &Config, session: &Session, input: &str) -> Self {
        Self(
            config
                .sections
                .iter()
                .filter_map(|section| match &section.source {
                    Source::Hook(hook) => {
                        cached(&section.name, hook, &session.id, &session.dir, input)
                            .map(|output| (section.name.clone(), output))
                    }
                    _ => None,
                })
                .collect(),
        )
    }

    pub fn get(&self, section: &SectionName) -> Option<&str> {
        self.0.get(section).map(String::as_str)
    }
}

/// The hook's last output, refreshed in the background once older than its `refresh`.
fn cached(
    section: &SectionName,
    hook: &Hook,
    session: &SessionId,
    dir: &str,
    input: &str,
) -> Option<String> {
    let cache = host::cache_dir()
        .join("hooks")
        .join(format!("{section}.{}", session.file_name()));
    let output = fs::read_to_string(&cache).ok();
    if age(&cache).is_none_or(|age| age >= hook.refresh) && !pending(&cache, hook.timeout) {
        let refresh = Refresh {
            cache: cache.clone(),
            section: section.to_string(),
            command: hook.command.clone(),
            dir: dir.to_owned(),
            timeout: hook.timeout.unsigned_abs(),
            input: input.to_owned(),
        };
        let _ = start(&refresh);
    }
    output.filter(|output| !output.is_empty())
}

fn start(refresh: &Refresh) -> std::io::Result<()> {
    if let Some(dir) = refresh.cache.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(beside(&refresh.cache, ".pending"), [])?;
    child::detach(SUBCOMMAND, refresh)
}

/// The detached child: runs the hook within its timeout and caches the first line it prints.
pub fn refresh(refresh: &Refresh) {
    let output = run(refresh).unwrap_or_default();
    let partial = beside(&refresh.cache, ".partial");
    if fs::write(&partial, output).is_ok() {
        let _ = fs::rename(&partial, &refresh.cache);
    }
    let _ = fs::remove_file(beside(&refresh.cache, ".pending"));
}

fn run(refresh: &Refresh) -> Option<String> {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(&refresh.command)
        .env("LINEA_SECTION", &refresh.section)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if Path::new(&refresh.dir).is_dir() {
        command.current_dir(&refresh.dir);
    }
    let mut child = command.spawn().ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(refresh.input.as_bytes());
    }
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        out
    });

    let deadline = Instant::now() + refresh.timeout;
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
    status
        .success()
        .then(|| first_line(&String::from_utf8_lossy(&out)))
}

fn first_line(out: &str) -> String {
    out.lines().next().unwrap_or_default().trim_end().to_owned()
}

fn age(path: &Path) -> Option<SignedDuration> {
    let modified = fs::metadata(path).and_then(|meta| meta.modified()).ok()?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default();
    SignedDuration::try_from(age).ok()
}

/// A refresh already running, unless it outlived its timeout.
fn pending(cache: &Path, timeout: SignedDuration) -> bool {
    age(&beside(cache, ".pending")).is_some_and(|age| age < timeout + SignedDuration::from_secs(1))
}

fn beside(cache: &Path, suffix: &str) -> PathBuf {
    let mut path = cache.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}
