//! Reads from the machine around the session: environment, terminal, settings, appearance.

use std::{collections::BTreeMap, env, ffi::OsString, fs::File, path::PathBuf};

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

/// An `XDG_*` base directory when it holds an absolute path, else the fallback.
fn xdg_base(value: Option<OsString>, fallback: Option<PathBuf>) -> Option<PathBuf> {
    value
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or(fallback)
}

/// The global linea.toml: `LINEA_CONFIG`, else `linea/linea.toml` under
/// `XDG_CONFIG_HOME`, else under `~/.config`, as on Linux.
pub fn global_config() -> Option<PathBuf> {
    match var("LINEA_CONFIG") {
        Some(path) => Some(PathBuf::from(path)),
        None => xdg_base(
            env::var_os("XDG_CONFIG_HOME"),
            dirs::home_dir().map(|home| home.join(".config")),
        )
        .map(|base| base.join("linea/linea.toml")),
    }
}

/// `linea` under `XDG_CACHE_HOME`, else under the platform's cache directory.
pub fn cache_dir() -> Option<PathBuf> {
    xdg_base(env::var_os("XDG_CACHE_HOME"), dirs::cache_dir()).map(|base| base.join("linea"))
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
    let path = match (claude.settings.as_deref(), var("CLAUDE_CONFIG_DIR")) {
        (Some(path), _) => match path.strip_prefix("~") {
            Ok(rest) => dirs::home_dir()?.join(rest),
            Err(_) => path.to_owned(),
        },
        (None, Some(dir)) => PathBuf::from(dir).join("settings.json"),
        (None, None) => dirs::home_dir()?.join(".claude/settings.json"),
    };
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
        Mode::Auto => system_appearance(),
        Mode::Dark => Appearance::Dark,
        Mode::Light => Appearance::Light,
    }
}

/// macOS always answers: the interface style is dark or it is light.
#[cfg(target_os = "macos")]
fn system_appearance() -> Appearance {
    interface_style()
}

/// The desktop's preference, else the terminal's background, else light.
#[cfg(not(target_os = "macos"))]
fn system_appearance() -> Appearance {
    desktop_appearance()
        .or_else(|| var("COLORFGBG").and_then(|colours| terminal_appearance(&colours)))
        .unwrap_or(Appearance::Light)
}

/// The background a terminal announces in `COLORFGBG` (`fg;bg`): ANSI 0–6 and 8 are dark.
#[cfg(any(not(target_os = "macos"), test))]
fn terminal_appearance(colours: &str) -> Option<Appearance> {
    match colours.rsplit(';').next()?.parse::<u8>().ok()? {
        0..=6 | 8 => Some(Appearance::Dark),
        7 | 9..=15 => Some(Appearance::Light),
        _ => None,
    }
}

/// The global `AppleInterfaceStyle` preference, as `defaults read -g` gives it:
/// `Dark`, or unset for light.
#[cfg(target_os = "macos")]
fn interface_style() -> Appearance {
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
    let dark = value
        .downcast::<CFString>()
        .is_some_and(|style| style == "Dark");
    if dark {
        Appearance::Dark
    } else {
        Appearance::Light
    }
}

/// The freedesktop settings portal's `color-scheme`: 1 prefers dark, 2 light, 0 neither.
#[cfg(target_os = "linux")]
fn desktop_appearance() -> Option<Appearance> {
    use zbus::{blocking::connection::Builder, zvariant::Value};

    fn scheme(value: &Value<'_>) -> Option<u32> {
        match value {
            Value::U32(scheme) => Some(*scheme),
            Value::Value(inner) => scheme(inner),
            _ => None,
        }
    }

    let connection = Builder::session()
        .ok()?
        .method_timeout(std::time::Duration::from_millis(200))
        .build()
        .ok()?;
    let read = |method| {
        connection
            .call_method(
                Some("org.freedesktop.portal.Desktop"),
                "/org/freedesktop/portal/desktop",
                Some("org.freedesktop.portal.Settings"),
                method,
                &("org.freedesktop.appearance", "color-scheme"),
            )
            .ok()
    };
    let reply = read("ReadOne").or_else(|| read("Read"))?;
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
    match scheme(&value)? {
        1 => Some(Appearance::Dark),
        2 => Some(Appearance::Light),
        _ => None,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn desktop_appearance() -> Option<Appearance> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorfgbg_names_the_background_last() {
        assert_eq!(terminal_appearance("15;0"), Some(Appearance::Dark));
        assert_eq!(terminal_appearance("0;default;15"), Some(Appearance::Light));
        assert_eq!(terminal_appearance("15;default"), None);
    }
}
