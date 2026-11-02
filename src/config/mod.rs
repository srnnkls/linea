//! The configuration as linea uses it: validated once when loaded, so every
//! section names a source, a colour for each appearance and a fit that holds.

mod load;
mod raw;

use std::{collections::BTreeMap, path::PathBuf};

use jiff::SignedDuration;
use serde::Deserialize;

use crate::template::Template;

pub use load::{DEFAULTS, load};

#[derive(Debug, Clone)]
pub struct Config {
    pub line: LineStyle,
    /// The sections drawn, in drawing order.
    pub sections: Vec<Section>,
    /// Every built-in section, drawn or not, for templates to read.
    pub built_ins: Vec<BuiltIn>,
    pub mode: Mode,
    pub herdr: Herdr,
    pub report: Report,
    pub claude: Claude,
}

#[derive(Debug, Clone)]
pub struct LineStyle {
    pub separator: String,
    /// Between a section's icon and its text.
    pub gap: String,
    /// Columns kept free at the right edge.
    pub margin: i64,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub source: Source,
    pub icon: Icon,
    pub shade: Shade,
    pub fit: Fit,
}

/// Where a section's text comes from.
#[derive(Debug, Clone)]
pub enum Source {
    BuiltIn(BuiltIn),
    Template(Template),
    Hook(Hook),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltIn {
    Model,
    Effort,
    Project,
    Branch,
    Pane,
    Duration,
    Cache { format: String },
    CacheMisses,
    Context,
}

/// A shell command whose first line of output is the section's text.
#[derive(Debug, Clone)]
pub struct Hook {
    pub command: String,
    pub refresh: SignedDuration,
    pub timeout: SignedDuration,
    /// Formats the command's output, which it reads as `output`.
    pub template: Option<Template>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Icon {
    Bare,
    Fixed(String),
    /// Steps from empty to full, picked by the context share.
    Steps(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// A section's colour under each appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shade {
    pub dark: Rgb,
    pub light: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Follows the macOS appearance.
    Auto,
    Dark,
    Light,
}

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

/// Where the pane section reads herdr from.
#[derive(Debug, Clone)]
pub struct Herdr {
    pub enabled: bool,
    /// Set to `1` inside a herdr pane.
    pub inside: String,
    pub pane: String,
    /// Names the socket that exists while herdr runs.
    pub socket: String,
}

/// Where model and effort changes are sent.
#[derive(Debug, Clone)]
pub struct Report {
    pub enabled: bool,
    pub command: Vec<String>,
    /// Printed by the command once it took the report.
    pub received: String,
    pub retry: SignedDuration,
    /// A variable that must be set for a report to go out.
    pub requires: Option<String>,
}

/// Where the effort level comes from when the session reports none.
#[derive(Debug, Clone)]
pub struct Claude {
    pub effort: String,
    /// The settings.json holding `effortLevel`; unset follows `CLAUDE_CONFIG_DIR`.
    pub settings: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}: {source}", path.display())]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{origin}: {}", source.message())]
    Parse {
        origin: String,
        source: toml::de::Error,
    },
    #[error("line.sections names `{0}`, which no [sections.{0}] defines")]
    Undefined(String),
    #[error("sections.{0} is no built-in section and has neither `run` nor `template`")]
    NoSource(String),
    #[error("sections.{section}.{key} applies only to {applies}")]
    Misplaced {
        section: String,
        key: &'static str,
        applies: &'static str,
    },
    #[error("sections.{0} needs a `format`")]
    NoFormat(String),
    #[error("sections.{section}.template: {message}")]
    Template { section: String, message: String },
    #[error("sections.{section} requires `{required}`, which line.sections leaves out")]
    Requires { section: String, required: String },
    #[error("sections.{section} uses colour `{colour}`, which a palette lacks")]
    Colour { section: String, colour: String },
    #[error("only one section may grow")]
    Growing,
}

const REFRESH: SignedDuration = SignedDuration::from_secs(30);
const TIMEOUT: SignedDuration = SignedDuration::from_secs(5);

impl Shade {
    pub fn under(self, appearance: Appearance) -> Rgb {
        match appearance {
            Appearance::Dark => self.dark,
            Appearance::Light => self.light,
        }
    }
}

impl Rgb {
    fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#').filter(|hex| hex.len() == 6)?;
        let channel = |at: usize| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok();
        Some(Self(channel(0)?, channel(2)?, channel(4)?))
    }
}

impl BuiltIn {
    pub const NAMES: [&str; 9] = [
        "model",
        "effort",
        "project",
        "branch",
        "pane",
        "duration",
        "cache",
        "cache_misses",
        "context",
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Effort => "effort",
            Self::Project => "project",
            Self::Branch => "branch",
            Self::Pane => "pane",
            Self::Duration => "duration",
            Self::Cache { .. } => "cache",
            Self::CacheMisses => "cache_misses",
            Self::Context => "context",
        }
    }

    fn named(name: &str, section: &raw::Section) -> Result<Option<Self>, Error> {
        let built_in = match name {
            "model" => Self::Model,
            "effort" => Self::Effort,
            "project" => Self::Project,
            "branch" => Self::Branch,
            "pane" => Self::Pane,
            "duration" => Self::Duration,
            "cache" => Self::Cache {
                format: section
                    .format
                    .clone()
                    .ok_or_else(|| Error::NoFormat(name.into()))?,
            },
            "cache_misses" => Self::CacheMisses,
            "context" => Self::Context,
            _ => return Ok(None),
        };
        Ok(Some(built_in))
    }
}

impl TryFrom<raw::Config> for Config {
    type Error = Error;

    fn try_from(raw: raw::Config) -> Result<Self, Error> {
        let drawn = &raw.line.sections;
        let sections = drawn
            .iter()
            .map(|name| {
                let section = raw
                    .sections
                    .get(name)
                    .ok_or_else(|| Error::Undefined(name.clone()))?;
                section
                    .enabled
                    .then(|| Section::new(name, section, &raw.palette, drawn))
                    .transpose()
            })
            .filter_map(Result::transpose)
            .collect::<Result<Vec<_>, Error>>()?;
        if sections
            .iter()
            .filter(|section| matches!(&section.fit, Fit::Ranked(claim) if claim.grow))
            .count()
            > 1
        {
            return Err(Error::Growing);
        }
        let built_ins = BuiltIn::NAMES
            .iter()
            .filter_map(|name| {
                raw.sections
                    .get(*name)
                    .map(|section| BuiltIn::named(name, section))
            })
            .filter_map(Result::transpose)
            .collect::<Result<_, _>>()?;

        Ok(Self {
            line: LineStyle {
                separator: raw.line.separator,
                gap: raw.line.gap,
                margin: raw.line.margin,
            },
            sections,
            built_ins,
            mode: raw.appearance.mode,
            herdr: Herdr {
                enabled: raw.herdr.enabled,
                inside: raw.herdr.inside,
                pane: raw.herdr.pane,
                socket: raw.herdr.socket,
            },
            report: Report {
                enabled: raw.report.enabled,
                command: raw.report.command,
                received: raw.report.received,
                retry: raw.report.retry,
                requires: raw.report.requires,
            },
            claude: Claude {
                effort: raw.claude.effort,
                settings: raw.claude.settings,
            },
        })
    }
}

impl Section {
    fn new(
        name: &str,
        section: &raw::Section,
        palettes: &raw::Palettes,
        drawn: &[String],
    ) -> Result<Self, Error> {
        let misplaced = |key, applies| Error::Misplaced {
            section: name.into(),
            key,
            applies,
        };
        if section.format.is_some() && name != "cache" {
            return Err(misplaced("format", "the cache section"));
        }
        if !section.icons.is_empty() && name != "context" {
            return Err(misplaced("icons", "the context section"));
        }
        if section.run.is_none() {
            if section.refresh.is_some() {
                return Err(misplaced("refresh", "a section with `run`"));
            }
            if section.timeout.is_some() {
                return Err(misplaced("timeout", "a section with `run`"));
            }
        }
        if let Fit::Ranked(Claim {
            requires: Some(required),
            ..
        }) = &section.fit
            && !drawn.contains(required)
        {
            return Err(Error::Requires {
                section: name.into(),
                required: required.clone(),
            });
        }

        let template = section
            .template
            .as_deref()
            .map(Template::parse)
            .transpose()
            .map_err(|message| Error::Template {
                section: name.into(),
                message,
            })?;
        let source = match (&section.run, template) {
            (Some(command), template) => Source::Hook(Hook {
                command: command.clone(),
                refresh: section.refresh.unwrap_or(REFRESH),
                timeout: section.timeout.unwrap_or(TIMEOUT),
                template,
            }),
            (None, Some(template)) => Source::Template(template),
            (None, None) => Source::BuiltIn(
                BuiltIn::named(name, section)?.ok_or_else(|| Error::NoSource(name.into()))?,
            ),
        };

        let icon = match (&section.icon, section.icons.is_empty()) {
            (Some(icon), _) if icon.is_empty() => Icon::Bare,
            (Some(icon), _) => Icon::Fixed(icon.clone()),
            (None, false) => Icon::Steps(section.icons.clone()),
            (None, true) => Icon::Bare,
        };

        Ok(Self {
            name: name.into(),
            source,
            icon,
            shade: shade(name, &section.colour, palettes)?,
            fit: section.fit.clone(),
        })
    }
}

fn shade(section: &str, colour: &raw::Colour, palettes: &raw::Palettes) -> Result<Shade, Error> {
    let named = |palette: &BTreeMap<String, Rgb>, name: &String| {
        palette.get(name).copied().ok_or_else(|| Error::Colour {
            section: section.into(),
            colour: name.clone(),
        })
    };
    match colour {
        raw::Colour::Literal(rgb) => Ok(Shade {
            dark: *rgb,
            light: *rgb,
        }),
        raw::Colour::Named(name) => Ok(Shade {
            dark: named(&palettes.dark, name)?,
            light: named(&palettes.light, name)?,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(extra: &str) -> Result<Config, Error> {
        let mut table: toml::Table = DEFAULTS.parse().unwrap();
        load::merge(&mut table, extra.parse().unwrap());
        let raw: raw::Config = toml::Value::Table(table).try_into().unwrap();
        Config::try_from(raw)
    }

    #[test]
    fn a_source_follows_run_then_template_then_the_name() {
        let config = with(
            "[line]\nsections = [\"a\", \"b\", \"model\"]\n\
             [sections.a]\nrun = \"true\"\ncolour = \"grey\"\nfit = 1\n\
             [sections.b]\ntemplate = \"x\"\ncolour = \"grey\"\nfit = 2",
        )
        .unwrap();
        assert!(matches!(config.sections[0].source, Source::Hook(_)));
        assert!(matches!(config.sections[1].source, Source::Template(_)));
        assert!(matches!(
            config.sections[2].source,
            Source::BuiltIn(BuiltIn::Model)
        ));
    }

    #[test]
    fn keys_outside_their_section_are_errors() {
        let error = |extra| with(extra).unwrap_err().to_string();
        assert_eq!(
            error("sections.model.format = \"%H\""),
            "sections.model.format applies only to the cache section"
        );
        assert_eq!(
            error("sections.model.icons = [\"a\"]"),
            "sections.model.icons applies only to the context section"
        );
        assert_eq!(
            error("sections.model.refresh = \"1s\""),
            "sections.model.refresh applies only to a section with `run`"
        );
    }

    #[test]
    fn a_section_without_a_source_is_an_error() {
        let error = |extra| with(extra).unwrap_err().to_string();
        assert_eq!(
            error("line.sections = [\"w\"]\nsections.w.colour = \"grey\"\nsections.w.fit = 1"),
            "sections.w is no built-in section and has neither `run` nor `template`"
        );
    }

    #[test]
    fn only_one_section_grows() {
        let error = with("sections.project.fit = { rank = 1, grow = true }").unwrap_err();
        assert!(matches!(error, Error::Growing));
    }
}
