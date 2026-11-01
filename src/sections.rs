//! What each section shows: built-in texts from the session, hook output, or a `text` template.

use std::collections::BTreeMap;

use jiff::{Timestamp, tz::TimeZone};
use serde_json::Value;

use crate::{
    config::{Colour, Config, Rgb, Section},
    line::{Appearance, Part},
    status::{Context, Effort, Status},
    template::Templates,
};

pub const BUILT_IN: [&str; 9] = [
    "model", "effort", "project", "branch", "pane", "duration", "cache", "misses", "context",
];

/// What the line shows beyond the session state itself.
#[derive(Debug, Clone)]
pub struct Surroundings {
    pub appearance: Appearance,
    pub root: String,
    pub branch: String,
    pub effort: Option<Effort>,
    pub pane: Option<String>,
    pub zone: TimeZone,
}

/// The configured sections that have something to show, in drawing order.
/// `hook` gives a section's output when it has a `run` command. A `text`
/// template takes the place of the built-in text, and of a hook's output,
/// which it reads as `output`; an empty result leaves the section out.
pub fn parts(
    config: &Config,
    status: &Status,
    around: &Surroundings,
    input: &Value,
    hook: impl Fn(&str, &Section) -> Option<String>,
) -> Result<Vec<Part>, String> {
    let palette = match around.appearance {
        Appearance::Dark => &config.palette.dark,
        Appearance::Light => &config.palette.light,
    };
    let format = |name: &str| config.sections.get(name).and_then(|s| s.format.as_deref());
    let built: BTreeMap<String, String> = BUILT_IN
        .iter()
        .map(|name| {
            let text = built_in(name, format(name), status, around).unwrap_or_default();
            ((*name).to_owned(), text)
        })
        .collect();
    let any_template = config
        .line
        .sections
        .iter()
        .any(|name| config.sections.get(name).is_some_and(|s| s.text.is_some()));
    let renderer = any_template.then(|| Templates::new(input, built.clone(), around.zone.clone()));

    let mut parts = Vec::new();
    for name in &config.line.sections {
        let Some(section) = config.sections.get(name).filter(|section| section.enabled) else {
            continue;
        };
        let output = section.run.as_ref().and_then(|_| hook(name, section));
        let text = match (&section.text, &renderer) {
            (Some(_), _) if section.run.is_some() && output.is_none() => None,
            (Some(source), Some(renderer)) => Some(
                renderer
                    .render(source, output.as_deref())
                    .map_err(|error| format!("sections.{name}: {error}"))?,
            )
            .filter(|text| !text.is_empty()),
            _ if section.run.is_some() => output,
            _ => built_in(name, section.format.as_deref(), status, around),
        };
        let Some(text) = text else { continue };
        let colour = match &section.colour {
            Colour::Literal(rgb) => *rgb,
            Colour::Named(name) => palette.get(name).copied().unwrap_or(Rgb(0x80, 0x80, 0x80)),
        };
        let icon = section
            .icon
            .clone()
            .or_else(|| step(&section.icons, status.context.percent))
            .unwrap_or_default();
        parts.push(Part {
            name: name.clone(),
            colour,
            icon,
            text,
            fit: section.fit.clone(),
        });
    }
    Ok(parts)
}

fn built_in(
    name: &str,
    format: Option<&str>,
    status: &Status,
    around: &Surroundings,
) -> Option<String> {
    let present = |text: String| Some(text).filter(|text| !text.is_empty());
    match name {
        "model" => Some(status.model.name.clone()),
        "effort" => around
            .effort
            .as_ref()
            .map(|effort| effort.as_str().to_owned()),
        "project" => present(around.root.clone()),
        "branch" => present(around.branch.clone()),
        "pane" => around.pane.clone(),
        "duration" => present(duration(status.elapsed_ms)),
        "cache" => present(cache(status, format.unwrap_or("%H:%M"), &around.zone)),
        "misses" => (status.cache.misses > 0).then(|| status.cache.misses.to_string()),
        "context" => Some(context(&status.context)),
        _ => None,
    }
}

/// The icon for a share: steps from empty to full, the last one only at 100%.
fn step(icons: &[String], percent: i64) -> Option<String> {
    let last = i64::try_from(icons.len()).ok()? - 1;
    if last < 0 {
        return None;
    }
    let at = (percent.min(100) * last + 50) / 100;
    let at = if at == last && percent < 100 {
        last - 1
    } else {
        at
    };
    icons.get(usize::try_from(at.clamp(0, last)).ok()?).cloned()
}

fn context(context: &Context) -> String {
    format!(
        "{}/{} ({}%)",
        human(context.used),
        human(context.size),
        context.percent
    )
}

pub fn human(n: f64) -> String {
    if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 1000.0 {
        format!("{:.0}k", n / 1000.0)
    } else {
        format!("{}", n.trunc())
    }
}

/// Hours and minutes; nothing before the first second.
pub fn duration(elapsed_ms: i64) -> String {
    let seconds = elapsed_ms / 1000;
    if seconds > 0 {
        format!("{:02}:{:02}", seconds / 3600, seconds % 3600 / 60)
    } else {
        String::new()
    }
}

fn cache(status: &Status, format: &str, zone: &TimeZone) -> String {
    if !status.cache.observed || status.cache.expires <= 0 {
        return String::new();
    }
    Timestamp::from_second(status.cache.expires)
        .map(|at| at.to_zoned(zone.clone()).strftime(format).to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::line;
    use serde_json::json;

    fn around(root: &str, branch: &str) -> Surroundings {
        Surroundings {
            appearance: Appearance::Dark,
            root: root.into(),
            branch: branch.into(),
            effort: Effort::new("high"),
            pane: None,
            zone: TimeZone::UTC,
        }
    }

    fn strip(line: &str) -> String {
        let mut out = String::new();
        let mut chars = line.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                chars.by_ref().find(|&c| c == 'm');
            } else {
                out.push(c);
            }
        }
        out
    }

    fn draw(config: &Config, status: &Status, around: &Surroundings, columns: i64) -> String {
        let parts = parts(config, status, around, &json!({}), |name, _| {
            Some(format!("<{name}>"))
        })
        .unwrap();
        strip(&line::render(&parts, columns, &config.line))
    }

    #[test]
    fn human_scales_tokens() {
        assert_eq!(human(999.0), "999");
        assert_eq!(human(45_200.0), "45k");
        assert_eq!(human(1_250_000.0), "1.2M");
    }

    #[test]
    fn the_pie_is_full_only_at_one_hundred() {
        let icons: Vec<String> = (0..9).map(|i| i.to_string()).collect();
        assert_eq!(step(&icons, 0).as_deref(), Some("0"));
        assert_eq!(step(&icons, 50).as_deref(), Some("4"));
        assert_eq!(step(&icons, 99).as_deref(), Some("7"));
        assert_eq!(step(&icons, 100).as_deref(), Some("8"));
        assert_eq!(step(&[], 50), None);
    }

    #[test]
    fn duration_shows_hours_and_minutes() {
        assert_eq!(duration(999), "");
        assert_eq!(duration(3_725_000), "01:02");
    }

    #[test]
    fn the_pane_follows_the_branch_when_present() {
        let config = Config::defaults();
        let status = Status::parse(&json!({"model": {"display_name": "Opus"}}));
        let mut herdr = around("dotfiles", "main");
        herdr.pane = Some("wM5:p1".into());
        let line = draw(&config, &status, &herdr, 124);
        assert!(line.contains("\u{f126} main  \u{eae3} wM5:p1  "), "{line}");
        assert!(!draw(&config, &status, &around("dotfiles", "main"), 124).contains('\u{eae3}'));
        assert!(!draw(&config, &status, &herdr, 34).contains("wM5"));
    }

    #[test]
    fn narrow_lines_drop_sections_and_cut_the_branch() {
        let config = Config::defaults();
        let status = Status::parse(&json!({
            "model": {"display_name": "Opus"},
            "context_window": {"total_input_tokens": 45_200, "used_percentage": 22},
            "cost": {"total_duration_ms": 3_725_000},
        }));
        let branch = around("dotfiles", "feature/long-branch-name");
        let wide = draw(&config, &status, &branch, 124);
        assert!(
            wide.contains("feature/long-branch-name") && wide.contains("01:02"),
            "{wide}"
        );
        let narrow = draw(&config, &status, &branch, 68);
        assert!(
            narrow.contains('…') && !narrow.contains("01:02"),
            "{narrow}"
        );
        assert!(!draw(&config, &status, &branch, 52).contains("feature"));
    }

    #[test]
    fn a_hook_section_draws_its_output() {
        let mut config = Config::defaults();
        let mut weather = config.sections["pane"].clone();
        weather.run = Some("true".into());
        weather.icon = Some("~".into());
        config.sections.insert("weather".into(), weather);
        config.line.sections.insert(1, "weather".into());
        let status = Status::parse(&json!({"model": {"display_name": "Opus"}}));
        let line = draw(&config, &status, &around("dotfiles", "main"), 124);
        assert!(line.starts_with("\u{f035b} Opus  ~ <weather>  "), "{line}");
    }

    #[test]
    fn a_text_template_reads_the_status_and_the_built_ins() {
        let mut config = Config::defaults();
        let mut cost = config.sections["pane"].clone();
        cost.text = Some("{{ linea.branch }} ${{ cost.total_cost_usd | round(2) }}".into());
        config.sections.insert("cost".into(), cost);
        config.line.sections.insert(1, "cost".into());
        let input = json!({"model": {"display_name": "Opus"}, "cost": {"total_cost_usd": 0.4567}});
        let status = Status::parse(&input);
        let parts = parts(
            &config,
            &status,
            &around("dotfiles", "main"),
            &input,
            |_, _| None,
        )
        .unwrap();
        let line = strip(&line::render(&parts, 124, &config.line));
        assert!(line.contains("\u{eae3} main $0.46  "), "{line}");
    }

    #[test]
    fn a_template_over_a_hook_waits_for_its_output() {
        let mut config = Config::defaults();
        let mut weather = config.sections["pane"].clone();
        weather.run = Some("true".into());
        weather.text = Some("{{ output }}°".into());
        config.sections.insert("weather".into(), weather);
        config.line.sections.insert(1, "weather".into());
        let input = json!({"model": {"display_name": "Opus"}});
        let status = Status::parse(&input);
        let around = around("dotfiles", "main");
        let draw = |out: Option<&str>| {
            let parts = parts(&config, &status, &around, &input, |_, _| {
                out.map(str::to_owned)
            })
            .unwrap();
            strip(&line::render(&parts, 124, &config.line))
        };
        assert!(draw(Some("12")).contains("12°"));
        assert!(!draw(None).contains('°'));
    }

    #[test]
    fn a_disabled_section_is_not_drawn() {
        let mut config = Config::defaults();
        config.sections.get_mut("effort").unwrap().enabled = false;
        let status = Status::parse(&json!({"model": {"display_name": "Opus"}}));
        assert!(!draw(&config, &status, &around("dotfiles", "main"), 124).contains("high"));
    }
}
