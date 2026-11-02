//! linea.toml as written: the serde mirror the domain `Config` is built from.

use std::{collections::BTreeMap, path::PathBuf};

use jiff::SignedDuration;
use serde::{Deserialize, Deserializer, de};

use super::{Mode, Rgb};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub line: Line,
    pub appearance: Appearance,
    pub palette: Palettes,
    pub sections: BTreeMap<String, Section>,
    pub herdr: Herdr,
    pub events: Events,
    pub claude: Claude,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub sections: Vec<String>,
    pub separator: String,
    pub gap: String,
    pub margin: u16,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Appearance {
    pub mode: Mode,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palettes {
    pub dark: BTreeMap<String, Rgb>,
    pub light: BTreeMap<String, Rgb>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    #[serde(default = "enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub icon: Option<Icon>,
    pub colour: Colour,
    pub fit: Fit,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub run: Option<String>,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default, deserialize_with = "duration")]
    pub refresh: Option<SignedDuration>,
    #[serde(default, deserialize_with = "duration")]
    pub timeout: Option<SignedDuration>,
    #[serde(default)]
    pub emit: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Icon {
    Fixed(String),
    Mapped(IconMap),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IconMap {
    pub state: String,
    #[serde(default)]
    pub at: Option<BTreeMap<String, String>>,
    #[serde(default, rename = "match")]
    pub matches: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub default: Option<String>,
}

/// `"exit"`, or `{ marker = "…" }`.
#[derive(Debug)]
pub enum Acknowledge {
    Exit,
    Marker(String),
}

/// `"always"`, a rank, or a claim table.
#[derive(Debug)]
pub enum Fit {
    Always,
    Rank(i64),
    Claim(Claim),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub rank: i64,
    #[serde(default)]
    pub grow: bool,
    #[serde(default)]
    pub min: Option<u16>,
    #[serde(default)]
    pub requires: Option<String>,
}

/// A palette name, or a literal `#rrggbb`.
#[derive(Debug, Clone)]
pub enum Colour {
    Named(String),
    Literal(Rgb),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Herdr {
    pub enabled: bool,
    pub inside: String,
    pub pane: String,
    pub socket: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Events {
    pub enabled: bool,
    pub command: Vec<String>,
    pub body: String,
    pub acknowledge: Acknowledge,
    #[serde(deserialize_with = "required_duration")]
    pub retry: SignedDuration,
    #[serde(default)]
    pub requires: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claude {
    pub effort: String,
    #[serde(default)]
    pub settings: Option<PathBuf>,
}

const fn enabled() -> bool {
    true
}

fn duration<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<SignedDuration>, D::Error> {
    required_duration(deserializer).map(Some)
}

fn required_duration<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<SignedDuration, D::Error> {
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

impl<'de> Deserialize<'de> for Acknowledge {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Marker {
            marker: String,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Word(String),
            Marker(Marker),
        }
        match Written::deserialize(deserializer)? {
            Written::Word(word) if word == "exit" => Ok(Self::Exit),
            Written::Word(word) => Err(de::Error::custom(format!(
                "acknowledge is \"exit\" or {{ marker = \"…\" }}, not `{word}`"
            ))),
            Written::Marker(Marker { marker }) => Ok(Self::Marker(marker)),
        }
    }
}

impl<'de> Deserialize<'de> for Fit {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Word(String),
            Rank(i64),
            Table(Claim),
        }
        match Written::deserialize(deserializer)? {
            Written::Word(word) if word == "always" => Ok(Self::Always),
            Written::Word(word) => Err(de::Error::custom(format!(
                "fit is \"always\", a rank or a table, not `{word}`"
            ))),
            Written::Rank(rank) => Ok(Self::Rank(rank)),
            Written::Table(claim) => Ok(Self::Claim(claim)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_reads_a_word_a_rank_or_a_table() {
        let fit = |text: &str| {
            toml::from_str::<BTreeMap<String, Fit>>(&format!("fit = {text}"))
                .map(|mut m| m.remove("fit").unwrap())
        };
        assert!(matches!(fit("\"always\"").unwrap(), Fit::Always));
        assert!(matches!(fit("3").unwrap(), Fit::Rank(3)));
        assert!(matches!(
            fit("{ rank = 1, grow = true, min = 8 }").unwrap(),
            Fit::Claim(Claim {
                grow: true,
                min: Some(8),
                ..
            })
        ));
        assert!(fit("\"never\"").is_err());
    }
}
