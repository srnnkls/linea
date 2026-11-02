//! The configuration as linea uses it. linea.toml is parsed into these types
//! once, at load: every drawn section has a source, a colour under each
//! appearance and a fit that holds, so nothing downstream checks again.

mod load;
mod raw;

use std::{collections::BTreeMap, fmt, path::PathBuf};

use jiff::SignedDuration;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub events: Events,
    pub claude: Claude,
}

#[derive(Debug, Clone)]
pub struct LineStyle {
    pub separator: String,
    /// Between a section's icon and its text.
    pub gap: String,
    /// Columns kept free at the right edge.
    pub margin: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SectionName(String);

#[derive(Debug, Clone)]
pub struct Section {
    pub name: SectionName,
    pub source: Source,
    pub icon: Icon,
    pub shade: Shade,
    pub fit: Fit,
    /// Sends an event when the section's text changes.
    pub emit: bool,
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

#[derive(Debug, Clone, PartialEq)]
pub enum Icon {
    Bare,
    Fixed(String),
    Mapped(IconMap),
}

/// An icon picked by the value of a named state.
#[derive(Debug, Clone, PartialEq)]
pub struct IconMap {
    /// A path into the status JSON or `linea`, such as `context_window.used_percentage`.
    pub state: String,
    pub rule: IconRule,
    /// For a state that is missing or matches no rule.
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IconRule {
    /// The icon of the highest threshold at or below a numeric state; thresholds ascend.
    At(Vec<(f64, String)>),
    /// The icon for exactly this value.
    Match(BTreeMap<String, String>),
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

/// How a section claims its columns. Lower ranks claim first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    /// Always drawn; its columns are taken before any ranked section.
    Always,
    /// Drawn while it fits.
    Ranked {
        rank: i64,
        requires: Option<SectionName>,
    },
    /// Holds up to `min` columns at its rank, then takes whatever is left once
    /// every rank is placed, cut short with an ellipsis.
    Grow {
        rank: i64,
        min: u16,
        requires: Option<SectionName>,
    },
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

/// Whether events go out when an emitting section's text changes, and where.
#[derive(Debug, Clone)]
pub enum Events {
    Off,
    On(Sink),
}

/// The command events go to.
#[derive(Debug, Clone)]
pub struct Sink {
    pub program: String,
    pub args: Vec<String>,
    /// Rendered as the event the command reads on stdin.
    pub body: Template,
    pub acknowledgement: Acknowledgement,
    pub retry: SignedDuration,
    /// A variable that must be set for an event to go out.
    pub requires: Option<String>,
}

/// How the events command signals that it took an event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Acknowledgement {
    /// Its output contains this text.
    Marker(String),
    /// It exits with status zero.
    ExitStatus,
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
    #[error("sections.{0}.icon takes either `at` or `match`")]
    IconRule(String),
    #[error("sections.{section}.icon.at: `{key}` is not a number")]
    Threshold { section: String, key: String },
    #[error("events.body: {0}")]
    Body(String),
    #[error("events are enabled but events.command names no program")]
    NoProgram,
    #[error("events are enabled but have no events.body")]
    NoBody,
}

const REFRESH: SignedDuration = SignedDuration::from_secs(30);
const TIMEOUT: SignedDuration = SignedDuration::from_secs(5);

impl SectionName {
    #[cfg(test)]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl fmt::Display for SectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

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

impl IconMap {
    /// The icon for a state's value.
    pub fn pick(&self, state: Option<&Value>) -> Option<&str> {
        let picked = match (&self.rule, state) {
            (IconRule::At(thresholds), Some(value)) => value.as_f64().and_then(|value| {
                thresholds
                    .iter()
                    .rev()
                    .find(|(threshold, _)| *threshold <= value)
                    .map(|(_, icon)| icon.as_str())
            }),
            (IconRule::Match(matches), Some(value)) => {
                let key = match value {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                };
                matches.get(&key).map(String::as_str)
            }
            (_, None) => None,
        };
        picked.or(self.default.as_deref())
    }
}

impl Fit {
    pub fn requires(&self) -> Option<&SectionName> {
        match self {
            Self::Always => None,
            Self::Ranked { requires, .. } | Self::Grow { requires, .. } => requires.as_ref(),
        }
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
            .filter(|section| matches!(section.fit, Fit::Grow { .. }))
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
            events: events(raw.events)?,
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
        if section.run.is_none() {
            if section.refresh.is_some() {
                return Err(misplaced("refresh", "a section with `run`"));
            }
            if section.timeout.is_some() {
                return Err(misplaced("timeout", "a section with `run`"));
            }
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

        let icon = match &section.icon {
            None => Icon::Bare,
            Some(raw::Icon::Fixed(icon)) if icon.is_empty() => Icon::Bare,
            Some(raw::Icon::Fixed(icon)) => Icon::Fixed(icon.clone()),
            Some(raw::Icon::Mapped(map)) => Icon::Mapped(icon_map(name, map)?),
        };

        Ok(Self {
            name: SectionName(name.into()),
            source,
            icon,
            shade: shade(name, &section.colour, palettes)?,
            fit: fit(name, &section.fit, drawn)?,
            emit: section.emit,
        })
    }
}

fn events(events: raw::Events) -> Result<Events, Error> {
    if !events.enabled {
        return Ok(Events::Off);
    }
    let mut command = events.command.into_iter();
    let program = command.next().ok_or(Error::NoProgram)?;
    let body = Template::parse(&events.body.ok_or(Error::NoBody)?).map_err(Error::Body)?;
    Ok(Events::On(Sink {
        program,
        args: command.collect(),
        body,
        acknowledgement: match events.acknowledge {
            raw::Acknowledge::Exit => Acknowledgement::ExitStatus,
            raw::Acknowledge::Marker(marker) => Acknowledgement::Marker(marker),
        },
        retry: events.retry,
        requires: events.requires,
    }))
}

fn fit(section: &str, fit: &raw::Fit, drawn: &[String]) -> Result<Fit, Error> {
    let requires = |required: &Option<String>| {
        required
            .as_ref()
            .map(|required| {
                if drawn.contains(required) {
                    Ok(SectionName(required.clone()))
                } else {
                    Err(Error::Requires {
                        section: section.into(),
                        required: required.clone(),
                    })
                }
            })
            .transpose()
    };
    match fit {
        raw::Fit::Always => Ok(Fit::Always),
        raw::Fit::Rank(rank) => Ok(Fit::Ranked {
            rank: *rank,
            requires: None,
        }),
        raw::Fit::Claim(claim) => match (claim.grow, claim.min) {
            (true, min) => Ok(Fit::Grow {
                rank: claim.rank,
                min: min.unwrap_or(0),
                requires: requires(&claim.requires)?,
            }),
            (false, None) => Ok(Fit::Ranked {
                rank: claim.rank,
                requires: requires(&claim.requires)?,
            }),
            (false, Some(_)) => Err(Error::Misplaced {
                section: section.into(),
                key: "fit.min",
                applies: "a section that grows",
            }),
        },
    }
}

fn icon_map(section: &str, map: &raw::IconMap) -> Result<IconMap, Error> {
    let rule = match (&map.at, &map.matches) {
        (Some(at), None) => {
            let mut thresholds = at
                .iter()
                .map(|(key, icon)| {
                    key.parse::<f64>()
                        .map(|threshold| (threshold, icon.clone()))
                        .map_err(|_| Error::Threshold {
                            section: section.into(),
                            key: key.clone(),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            thresholds.sort_by(|a, b| a.0.total_cmp(&b.0));
            IconRule::At(thresholds)
        }
        (None, Some(matches)) => IconRule::Match(matches.clone()),
        _ => return Err(Error::IconRule(section.into())),
    };
    Ok(IconMap {
        state: map.state.clone(),
        rule,
        default: map.default.clone(),
    })
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
    use serde_json::json;

    fn with(extra: &str) -> Result<Config, Error> {
        let mut table: toml::Table = DEFAULTS.parse().unwrap();
        load::merge(&mut table, extra.parse().unwrap());
        let raw: raw::Config =
            toml::Value::Table(table)
                .try_into()
                .map_err(|source| Error::Parse {
                    origin: "linea.toml".into(),
                    source,
                })?;
        Config::try_from(raw)
    }

    fn error(extra: &str) -> String {
        with(extra).unwrap_err().to_string()
    }

    #[test]
    fn a_source_follows_run_then_template_then_the_name() {
        let config = with(
            "[line]\nsections = [\"a\", \"b\", \"model\"]\n\
             [sections.a]\nrun = \"true\"\ntemplate = \"<{{ output }}>\"\ncolour = \"grey\"\nfit = 1\n\
             [sections.b]\ntemplate = \"x\"\ncolour = \"grey\"\nfit = 2",
        )
        .unwrap();
        assert!(
            matches!(&config.sections[0].source, Source::Hook(hook) if hook.template.is_some())
        );
        assert!(matches!(config.sections[1].source, Source::Template(_)));
        assert!(matches!(
            config.sections[2].source,
            Source::BuiltIn(BuiltIn::Model)
        ));
    }

    #[test]
    fn a_section_without_a_source_is_an_error() {
        assert_eq!(
            error("line.sections = [\"w\"]\nsections.w.colour = \"grey\"\nsections.w.fit = 1"),
            "sections.w is no built-in section and has neither `run` nor `template`"
        );
    }

    #[test]
    fn keys_outside_their_section_are_errors() {
        assert_eq!(
            error("sections.model.format = \"%H\""),
            "sections.model.format applies only to the cache section"
        );
        assert_eq!(
            error("sections.model.refresh = \"1s\""),
            "sections.model.refresh applies only to a section with `run`"
        );
        assert_eq!(
            error("sections.project.fit = { rank = 1, min = 8 }"),
            "sections.project.fit.min applies only to a section that grows"
        );
    }

    #[test]
    fn a_requirement_must_be_drawn_and_only_one_section_grows() {
        assert_eq!(
            error("line.sections = [\"model\", \"branch\"]"),
            "sections.branch requires `project`, which line.sections leaves out"
        );
        assert!(matches!(
            with("sections.project.fit = { rank = 1, grow = true }"),
            Err(Error::Growing)
        ));
    }

    #[test]
    fn an_icon_map_picks_by_threshold_or_match_and_falls_back() {
        let at = IconMap {
            state: "p".into(),
            rule: IconRule::At(vec![
                (0.0, "empty".into()),
                (50.0, "half".into()),
                (100.0, "full".into()),
            ]),
            default: Some("unknown".into()),
        };
        assert_eq!(at.pick(Some(&json!(49.9))), Some("empty"));
        assert_eq!(at.pick(Some(&json!(50))), Some("half"));
        assert_eq!(at.pick(Some(&json!(100))), Some("full"));
        assert_eq!(at.pick(Some(&json!(-1))), Some("unknown"));
        assert_eq!(at.pick(Some(&json!("high"))), Some("unknown"));
        assert_eq!(at.pick(None), Some("unknown"));

        let matches = IconMap {
            state: "e".into(),
            rule: IconRule::Match(BTreeMap::from([("high".into(), "H".into())])),
            default: None,
        };
        assert_eq!(matches.pick(Some(&json!("high"))), Some("H"));
        assert_eq!(matches.pick(Some(&json!("low"))), None);
    }

    #[test]
    fn an_icon_map_needs_exactly_one_rule_with_numeric_thresholds() {
        assert_eq!(
            error("sections.model.icon = { state = \"x\" }"),
            "sections.model.icon takes either `at` or `match`"
        );
        assert_eq!(
            error("sections.model.icon = { state = \"x\", at = { low = \"a\" } }"),
            "sections.model.icon.at: `low` is not a number"
        );
    }

    #[test]
    fn events_are_off_unless_enabled_with_a_program_and_a_body() {
        assert!(matches!(with("").unwrap().events, Events::Off));
        let on =
            "[events]\nenabled = true\ncommand = [\"notify\", \"-q\"]\nbody = \"{{ model.id }}\"\n";
        let Events::On(sink) = with(on).unwrap().events else {
            panic!("events are on")
        };
        assert_eq!(
            (sink.program.as_str(), sink.args.as_slice()),
            ("notify", ["-q".to_owned()].as_slice())
        );
        assert_eq!(sink.acknowledgement, Acknowledgement::ExitStatus);
        assert_eq!(
            error("events.enabled = true\nevents.body = \"x\""),
            "events are enabled but events.command names no program"
        );
        assert_eq!(
            error("events.enabled = true\nevents.command = [\"notify\"]"),
            "events are enabled but have no events.body"
        );
        let marked = format!("{on}acknowledge = {{ marker = \"ok\" }}");
        assert!(matches!(
            with(&marked).unwrap().events,
            Events::On(Sink {
                acknowledgement: Acknowledgement::Marker(_),
                ..
            })
        ));
    }
}
