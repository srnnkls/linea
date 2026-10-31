//! Reads from the machine around the session: terminal, repository, settings, appearance.

use std::{env, fs::File, path::PathBuf};

use serde_json::Value;

use crate::{line::Appearance, status::Effort};

const MARGIN: i64 = 4;

/// Text columns for the line: `COLUMNS`, else the controlling terminal, else 80, less a margin.
pub fn columns() -> i64 {
    let columns = match var("COLUMNS") {
        Some(columns) => columns.trim().parse().unwrap_or(0),
        None => File::open("/dev/tty")
            .ok()
            .and_then(|tty| rustix::termios::tcgetwinsize(&tty).ok())
            .map_or(80, |size| i64::from(size.ws_col)),
    };
    columns - MARGIN
}

/// The project named by the repository's main worktree, else by the directory itself.
pub fn workspace(dir: &str) -> (String, String) {
    let Ok(repo) = gix::discover_with_environment_overrides(dir) else {
        return (basename(dir).to_owned(), String::new());
    };
    let common =
        std::fs::canonicalize(repo.common_dir()).unwrap_or_else(|_| repo.common_dir().to_owned());
    let root = common.parent().map_or_else(
        || "/".to_owned(),
        |parent| basename(&parent.to_string_lossy()).to_owned(),
    );
    let branch = repo
        .head_name()
        .ok()
        .flatten()
        .and_then(|name| {
            name.as_bstr()
                .strip_prefix(b"refs/heads/")
                .map(|b| String::from_utf8_lossy(b).into_owned())
        })
        .unwrap_or_default();
    (root, branch)
}

/// The configured level when the session reports none.
pub fn effort() -> Option<Effort> {
    if let Some(level) = var("CLAUDE_CODE_EFFORT_LEVEL") {
        return Effort::new(level);
    }
    let settings = std::fs::read(config_dir().join("settings.json")).ok()?;
    let settings: Value = serde_json::from_slice(&settings).ok()?;
    let level = match settings.get("effortLevel") {
        None | Some(Value::Null | Value::Bool(false)) => "default".to_owned(),
        Some(Value::String(level)) => level.clone(),
        Some(level) => level.to_string(),
    };
    Effort::new(level)
}

pub fn config_dir() -> PathBuf {
    var("CLAUDE_CONFIG_DIR").map_or_else(|| home().join(".claude"), PathBuf::from)
}

fn home() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

/// An environment variable, with an empty value counting as unset.
pub fn var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

/// Whether an executable of this name is on `PATH`.
pub fn on_path(name: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    env::var_os("PATH").is_some_and(|path| {
        env::split_paths(&path).any(|dir| {
            std::fs::metadata(dir.join(name))
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
    })
}

/// The last path component, as `basename` gives it.
fn basename(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return if path.is_empty() { "" } else { "/" };
    }
    trimmed.rsplit('/').next().unwrap_or(trimmed)
}

#[cfg(target_os = "macos")]
pub fn appearance() -> Appearance {
    use core_foundation::{
        base::{CFType, TCFType},
        string::CFString,
    };
    use core_foundation_sys::preferences::{
        CFPreferencesCopyAppValue, kCFPreferencesAnyApplication,
    };

    let key = CFString::from_static_string("AppleInterfaceStyle");
    // SAFETY: both arguments are valid CFStrings for the duration of the call.
    let value = unsafe {
        CFPreferencesCopyAppValue(key.as_concrete_TypeRef(), kCFPreferencesAnyApplication)
    };
    if value.is_null() {
        return Appearance::Light;
    }
    // SAFETY: a non-null result of a Copy function is owned by the caller.
    let value = unsafe { CFType::wrap_under_create_rule(value) };
    match value.downcast::<CFString>() {
        Some(style) if style == "Dark" => Appearance::Dark,
        _ => Appearance::Light,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn appearance() -> Appearance {
    Appearance::Light
}

#[cfg(test)]
mod tests {
    use super::basename;

    #[test]
    fn basename_follows_the_shell() {
        assert_eq!(basename("/a/b/"), "b");
        assert_eq!(basename("/"), "/");
        assert_eq!(basename("."), ".");
        assert_eq!(basename(""), "");
    }
}
