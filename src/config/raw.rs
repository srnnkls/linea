//! linea.toml as written: the serde mirror the domain `Config` is built from.

use std::{collections::BTreeMap, path::PathBuf};

use jiff::SignedDuration;
use serde::{Deserialize, Deserializer, de};

use super::{Claim, Fit, Mode, Rgb};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub line: Line,
    pub appearance: Appearance,
    pub palette: Palettes,
    pub sections: BTreeMap<String, Section>,
    pub herdr: Herdr,
    pub report: Report,
    pub claude: Claude,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub sections: Vec<String>,
    pub separator: String,
    pub gap: String,
    pub margin: i64,
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
    pub icon: Option<String>,
    #[serde(default)]
    pub icons: Vec<String>,
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
pub struct Report {
    pub enabled: bool,
    pub command: Vec<String>,
    pub received: String,
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
            Written::Rank(rank) => Ok(Self::Ranked(Claim {
                rank,
                grow: false,
                min: 0,
                requires: None,
            })),
            Written::Table(claim) => Ok(Self::Ranked(claim)),
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
}
