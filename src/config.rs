//! linea.toml: the built-in defaults, then the global file, then the project
//! files from the outermost directory inwards, `linea.local.toml` after
//! `linea.toml` in each.

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use jiff::SignedDuration;
use serde::{Deserialize, Deserializer, de};
use toml::{Table, Value};

pub const DEFAULTS: &str = include_str!("../assets/config/default.toml");

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub line: Line,
    pub appearance: AppearanceConfig,
    pub palette: Palettes,
    pub sections: BTreeMap<String, Section>,
    pub herdr: Herdr,
    pub report: Report,
    pub claude: Claude,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceConfig {
    pub mode: Mode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Auto,
    Dark,
    Light,
}

/// Where the pane section reads herdr from.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Herdr {
    pub enabled: bool,
    /// Set to `1` inside a herdr pane.
    pub inside: String,
    pub pane: String,
    /// Names the socket that exists while herdr runs.
    pub socket: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub enabled: bool,
    pub command: Vec<String>,
    /// Printed by the command once it took the report.
    pub received: String,
    #[serde(deserialize_with = "duration")]
    pub retry: SignedDuration,
    /// A variable that must be set for a report to go out.
    #[serde(default)]
    pub requires: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claude {
    /// The variable holding an effort level the session does not report.
    pub effort: String,
    /// The settings.json to read `effortLevel` from; unset follows `CLAUDE_CONFIG_DIR`.
    #[serde(default)]
    pub settings: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub sections: Vec<String>,
    pub separator: String,
    pub gap: String,
    pub margin: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palettes {
    pub dark: BTreeMap<String, Rgb>,
    pub light: BTreeMap<String, Rgb>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    #[serde(default = "enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub icon: Option<String>,
    /// Icons stepping with a share, as the context pie does.
    #[serde(default)]
    pub icons: Vec<String>,
    pub colour: Colour,
    pub fit: Fit,
    /// An strftime format, for sections that show a time.
    #[serde(default)]
    pub format: Option<String>,
    /// A shell command whose first line of output is the section's text.
    #[serde(default)]
    pub run: Option<String>,
    #[serde(default = "refresh", deserialize_with = "duration")]
    pub refresh: SignedDuration,
    #[serde(default = "timeout", deserialize_with = "duration")]
    pub timeout: SignedDuration,
}

/// A palette name, or a literal `#rrggbb`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Colour {
    Named(String),
    Literal(Rgb),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// How a section claims its columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    /// Always drawn; its columns are taken before any ranked section.
    Always,
    Ranked(Claim),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    /// Lower ranks claim their columns first.
    pub rank: i64,
    /// Holds `min` columns at its rank, then takes whatever is left once every rank is placed.
    #[serde(default)]
    pub grow: bool,
    #[serde(default)]
    pub min: i64,
    /// Drawn only when this section is.
    #[serde(default)]
    pub requires: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    Read(PathBuf, std::io::Error),
    Parse(String, toml::de::Error),
    Invalid(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(path, error) => write!(f, "{}: {error}", path.display()),
            Self::Parse(origin, error) => write!(f, "{origin}: {}", error.message()),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

impl Config {
    /// The defaults with every file that applies to `dir` merged over them.
    pub fn load(global: Option<&Path>, dir: &Path) -> Result<Self, Error> {
        let mut merged = parse(DEFAULTS, "defaults")?;
        for path in global
            .into_iter()
            .map(Path::to_owned)
            .chain(project_files(dir))
        {
            match std::fs::read_to_string(&path) {
                Ok(text) => merge(&mut merged, parse(&text, &path.display().to_string())?),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(Error::Read(path, error)),
            }
        }
        let config: Self = Value::Table(merged)
            .try_into()
            .map_err(|error| Error::Parse("linea.toml".into(), error))?;
        config.validate()?;
        Ok(config)
    }

    #[cfg(test)]
    pub fn defaults() -> Self {
        Self::load(None, Path::new("/nonexistent")).expect("the built-in defaults are valid")
    }

    fn validate(&self) -> Result<(), Error> {
        for name in &self.line.sections {
            let section = self.sections.get(name).ok_or_else(|| {
                Error::Invalid(format!(
                    "line.sections names `{name}`, which no [sections.{name}] defines"
                ))
            })?;
            if section.run.is_none() && !crate::sections::BUILT_IN.contains(&name.as_str()) {
                return Err(Error::Invalid(format!(
                    "sections.{name} is no built-in section and has no `run`"
                )));
            }
            if let Fit::Ranked(Claim {
                requires: Some(required),
                ..
            }) = &section.fit
                && !self.line.sections.contains(required)
            {
                return Err(Error::Invalid(format!(
                    "sections.{name} requires `{required}`, which line.sections leaves out"
                )));
            }
            for palette in [&self.palette.dark, &self.palette.light] {
                if let Colour::Named(colour) = &section.colour
                    && !palette.contains_key(colour)
                {
                    return Err(Error::Invalid(format!(
                        "sections.{name} uses colour `{colour}`, which a palette lacks"
                    )));
                }
            }
        }
        let growing = self
            .sections
            .values()
            .filter(|section| matches!(&section.fit, Fit::Ranked(claim) if claim.grow))
            .count();
        if growing > 1 {
            return Err(Error::Invalid("only one section may grow".into()));
        }
        Ok(())
    }
}

/// `linea.toml` and `linea.local.toml` in every directory from the root down to `dir`.
fn project_files(dir: &Path) -> impl Iterator<Item = PathBuf> {
    let mut dirs: Vec<&Path> = dir.ancestors().collect();
    dirs.reverse();
    dirs.into_iter()
        .flat_map(|dir| [dir.join("linea.toml"), dir.join("linea.local.toml")])
        .collect::<Vec<_>>()
        .into_iter()
}

fn parse(text: &str, origin: &str) -> Result<Table, Error> {
    text.parse::<Table>()
        .map_err(|error| Error::Parse(origin.into(), error))
}

/// Tables merge key by key; any other value replaces the one beneath it.
fn merge(base: &mut Table, over: Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(base)), Value::Table(over)) => merge(base, over),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

const fn enabled() -> bool {
    true
}

const fn refresh() -> SignedDuration {
    SignedDuration::from_secs(30)
}

const fn timeout() -> SignedDuration {
    SignedDuration::from_secs(5)
}

fn duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SignedDuration, D::Error> {
    let text = String::deserialize(deserializer)?;
    text.parse().map_err(de::Error::custom)
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text)
            .ok_or_else(|| de::Error::custom(format!("`{text}` is not a #rrggbb colour")))
    }
}

impl Rgb {
    fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#').filter(|hex| hex.len() == 6)?;
        let channel = |at: usize| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok();
        Some(Self(channel(0)?, channel(2)?, channel(4)?))
    }
}

impl<'de> Deserialize<'de> for Colour {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text.starts_with('#') {
            Rgb::parse(&text)
                .map(Self::Literal)
                .ok_or_else(|| de::Error::custom(format!("`{text}` is not a #rrggbb colour")))
        } else {
            Ok(Self::Named(text))
        }
    }
}

impl<'de> Deserialize<'de> for Fit {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Word(String),
            Claim(i64),
            Table(Claim),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Word(word) if word == "always" => Ok(Self::Always),
            Raw::Word(word) => Err(de::Error::custom(format!(
                "fit is \"always\", a rank or a table, not `{word}`"
            ))),
            Raw::Claim(rank) => Ok(Self::Ranked(Claim {
                rank,
                grow: false,
                min: 0,
                requires: None,
            })),
            Raw::Table(claim) => Ok(Self::Ranked(claim)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> Table {
        text.parse().unwrap()
    }

    #[test]
    fn the_defaults_load() {
        let config = Config::defaults();
        assert_eq!(config.line.sections.len(), 9);
        assert_eq!(config.sections["context"].icons.len(), 9);
    }

    #[test]
    fn tables_merge_and_arrays_replace() {
        let mut base = table(
            "[line]\nsections = [\"a\", \"b\"]\nmargin = 4\n[palette.dark]\nred = \"#ff0000\"",
        );
        merge(
            &mut base,
            table("[line]\nsections = [\"c\"]\n[palette.dark]\nblue = \"#0000ff\""),
        );
        assert_eq!(base["line"]["sections"].as_array().unwrap().len(), 1);
        assert_eq!(base["line"]["margin"].as_integer(), Some(4));
        assert_eq!(base["palette"]["dark"].as_table().unwrap().len(), 2);
    }

    #[test]
    fn project_files_run_outermost_first() {
        let files: Vec<_> = project_files(Path::new("/a/b")).collect();
        assert_eq!(files.first().unwrap(), Path::new("/linea.toml"));
        assert_eq!(files.last().unwrap(), Path::new("/a/b/linea.local.toml"));
    }

    #[test]
    fn fit_reads_a_word_a_rank_or_a_table() {
        let fit = |text: &str| {
            toml::from_str::<BTreeMap<String, Fit>>(&format!("fit = {text}"))
                .map(|mut m| m.remove("fit").unwrap())
        };
        assert_eq!(fit("\"always\"").unwrap(), Fit::Always);
        assert!(matches!(
            fit("3").unwrap(),
            Fit::Ranked(Claim {
                rank: 3,
                grow: false,
                ..
            })
        ));
        assert!(matches!(
            fit("{ rank = 1, grow = true, min = 8 }").unwrap(),
            Fit::Ranked(Claim {
                grow: true,
                min: 8,
                ..
            })
        ));
        assert!(fit("\"never\"").is_err());
    }

    #[test]
    fn an_unknown_section_is_an_error() {
        let dir = std::env::temp_dir().join(format!("linea-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("linea.toml"),
            "[line]\nsections = [\"model\", \"weather\"]",
        )
        .unwrap();
        let error = Config::load(None, &dir).unwrap_err().to_string();
        assert!(error.contains("weather"), "{error}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
