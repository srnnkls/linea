//! Reads from the machine around the session: environment, terminal, settings, appearance.

use std::{collections::BTreeMap, env, fs::File, path::PathBuf};

use jiff::tz::TimeZone;
use serde_json::Value;

use crate::{
    config::{self, Appearance, Mode},
    session::Effort,
};

/// An environment variable, with an empty value counting as unset.
pub fn var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

pub fn home() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

/// The global linea.toml: `LINEA_CONFIG`, else under the XDG config home.
pub fn global_config() -> PathBuf {
    var("LINEA_CONFIG").map_or_else(
        || {
            var("XDG_CONFIG_HOME")
                .map_or_else(|| home().join(".config"), PathBuf::from)
                .join("linea/linea.toml")
        },
        PathBuf::from,
    )
}

pub fn cache_dir() -> PathBuf {
    var("XDG_CACHE_HOME")
        .map_or_else(|| home().join(".cache"), PathBuf::from)
        .join("linea")
}

pub fn temp_dir() -> String {
    let dir = var("TMPDIR").unwrap_or_else(|| "/tmp".into());
    dir.strip_suffix('/').unwrap_or(&dir).to_owned()
}

/// Where the line is drawn.
#[derive(Debug, Clone)]
pub struct Terminal {
    pub columns: i64,
    pub appearance: Appearance,
    pub zone: TimeZone,
}

impl Terminal {
    pub fn observe(mode: Mode) -> Self {
        Self {
            columns: columns(),
            appearance: appearance(mode),
            zone: TimeZone::system(),
        }
    }
}

/// The herdr pane the session runs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pane(pub String);

impl Pane {
    /// The pane, while the session runs inside a herdr that is up.
    pub fn observe(herdr: &config::Herdr) -> Option<Self> {
        let inside = var(&herdr.inside).is_some_and(|flag| flag == "1");
        let live = var(&herdr.socket).is_some_and(|socket| std::path::Path::new(&socket).exists());
        if herdr.enabled && inside && live {
            var(&herdr.pane).map(Self)
        } else {
            None
        }
    }
}

/// The environment, for templates to read; an entry that is not UTF-8 is left out.
pub fn vars() -> BTreeMap<String, String> {
    env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
        .collect()
}

/// Terminal columns: `COLUMNS`, else the controlling terminal, else 80.
fn columns() -> i64 {
    match var("COLUMNS") {
        Some(columns) => columns.trim().parse().unwrap_or(0),
        None => File::open("/dev/tty")
            .ok()
            .and_then(|tty| rustix::termios::tcgetwinsize(&tty).ok())
            .map_or(80, |size| i64::from(size.ws_col)),
    }
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

/// The configured effort level, for a session that reports none.
pub fn effort(claude: &config::Claude) -> Option<Effort> {
    if let Some(level) = var(&claude.effort) {
        return Effort::new(level);
    }
    let path = claude.settings.as_deref().map_or_else(
        || {
            var("CLAUDE_CONFIG_DIR")
                .map_or_else(|| home().join(".claude"), PathBuf::from)
                .join("settings.json")
        },
        |path| match path.strip_prefix("~") {
            Ok(rest) => home().join(rest),
            Err(_) => path.to_owned(),
        },
    );
    let settings: Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    let level = match settings.get("effortLevel") {
        None | Some(Value::Null | Value::Bool(false)) => "default".to_owned(),
        Some(Value::String(level)) => level.clone(),
        Some(level) => level.to_string(),
    };
    Effort::new(level)
}

fn appearance(mode: Mode) -> Appearance {
    match mode {
        Mode::Auto if interface_style().as_deref() == Some("Dark") => Appearance::Dark,
        Mode::Auto | Mode::Light => Appearance::Light,
        Mode::Dark => Appearance::Dark,
    }
}

/// The global `AppleInterfaceStyle` preference, as `defaults read -g` gives it.
#[cfg(target_os = "macos")]
fn interface_style() -> Option<String> {
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
        return None;
    }
    // SAFETY: a non-null result of a Copy function is owned by the caller.
    let value = unsafe { CFType::wrap_under_create_rule(value) };
    value.downcast::<CFString>().map(|style| style.to_string())
}

#[cfg(not(target_os = "macos"))]
fn interface_style() -> Option<String> {
    None
}
