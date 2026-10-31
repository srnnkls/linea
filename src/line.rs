use jiff::{Timestamp, tz::TimeZone};

use crate::status::{Context, Effort, Status};

const ICON_WIDTH: i64 = 1;
const GAP: &str = " ";
const SEP: &str = "  ";
const BRANCH_MIN: i64 = 24;
const RESET: &str = "\x1b[0m";

/// Context pie glyphs, empty to full in eighths.
const SLICES: [char; 9] = [
    '\u{f0130}',
    '\u{f0a9e}',
    '\u{f0a9f}',
    '\u{f0aa0}',
    '\u{f0aa1}',
    '\u{f0aa2}',
    '\u{f0aa3}',
    '\u{f0aa4}',
    '\u{f0aa5}',
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(u8, u8, u8);

/// Recessed doom-one and doom-one-light hues and greys, beside Claude Code's own orange.
#[derive(Debug, Clone, Copy)]
struct Palette {
    orange: Rgb,
    blue: Rgb,
    green: Rgb,
    red: Rgb,
    grey: Rgb,
}

impl Palette {
    fn of(appearance: Appearance) -> Self {
        let orange = Rgb(0xd7, 0x77, 0x57);
        match appearance {
            Appearance::Dark => Self {
                orange,
                blue: Rgb(0x6f, 0x9c, 0xc4),
                green: Rgb(0x8a, 0xa3, 0x6c),
                red: Rgb(0xb0, 0x7c, 0x7a),
                grey: Rgb(0x7f, 0x85, 0x8b),
            },
            Appearance::Light => Self {
                orange,
                blue: Rgb(0x5f, 0x82, 0xc4),
                green: Rgb(0x5f, 0x8f, 0x5e),
                red: Rgb(0xa8, 0x70, 0x6b),
                grey: Rgb(0x8a, 0x8e, 0x93),
            },
        }
    }
}

/// What the line shows beyond the session state itself.
#[derive(Debug, Clone)]
pub struct Surroundings {
    pub appearance: Appearance,
    pub columns: i64,
    pub root: String,
    pub branch: String,
    pub effort: Option<Effort>,
    pub zone: TimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    colour: Rgb,
    glyph: char,
    text: String,
}

impl Field {
    fn new(colour: Rgb, glyph: char, text: impl Into<String>) -> Self {
        Self {
            colour,
            glyph,
            text: text.into(),
        }
    }

    /// A field with no text shows nothing.
    fn optional(colour: Rgb, glyph: char, text: impl Into<String>) -> Option<Self> {
        Some(Self::new(colour, glyph, text)).filter(|field| !field.text.is_empty())
    }

    /// Columns on screen: icon, gap and text.
    fn width(&self) -> i64 {
        ICON_WIDTH + len(GAP) + len(&self.text)
    }

    fn render(&self) -> String {
        let Rgb(r, g, b) = self.colour;
        format!(
            "\x1b[38;2;{r};{g};{b}m{}{GAP}{}{RESET}",
            self.glyph, self.text
        )
    }
}

/// Columns taken so far against the columns available.
#[derive(Debug, Clone, Copy)]
struct Budget {
    columns: i64,
    taken: i64,
}

impl Budget {
    fn admit(self, field: Option<Field>, reserve: i64) -> (Self, Option<Field>) {
        match field {
            Some(field) if self.taken + len(SEP) + field.width() + reserve <= self.columns => (
                Self {
                    taken: self.taken + len(SEP) + field.width(),
                    ..self
                },
                Some(field),
            ),
            _ => (self, None),
        }
    }

    /// Text columns left for one more field.
    fn room(self) -> i64 {
        self.columns - self.taken - len(SEP) - ICON_WIDTH - len(GAP)
    }
}

/// Model and token usage always show. Project and effort come next, then the
/// branch at up to `BRANCH_MIN` columns, then duration and cache while they
/// fit; the branch grows into whatever room is left.
pub fn render(status: &Status, around: &Surroundings) -> String {
    let palette = Palette::of(around.appearance);
    let context = context_text(&status.context);
    let model = Field::new(palette.orange, '\u{f035b}', &status.model.name);
    let pie = Field::new(palette.grey, SLICES[slice(status.context.percent)], context);
    let misses = (status.cache.misses > 0).then(|| status.cache.misses.to_string());
    let misses = misses.and_then(|m| Field::optional(palette.red, '\u{f1635}', m));

    let taken = model.width()
        + len(SEP)
        + pie.width()
        + misses.as_ref().map_or(0, |m| len(SEP) + m.width());
    let budget = Budget {
        columns: around.columns,
        taken,
    };

    let (budget, root) = budget.admit(Field::optional(palette.blue, '\u{ea83}', &around.root), 0);
    let effort = around
        .effort
        .as_ref()
        .map(Effort::as_str)
        .unwrap_or_default();
    let (budget, effort) = budget.admit(Field::optional(palette.grey, '\u{f140b}', effort), 0);
    let branch = root
        .as_ref()
        .and_then(|_| Field::optional(palette.green, '\u{f126}', &around.branch));

    let reserve = branch.as_ref().map_or(0, |branch| {
        let held = budget.room().min(BRANCH_MIN).min(len(&branch.text));
        if held >= 4 {
            len(SEP) + ICON_WIDTH + len(GAP) + held
        } else {
            0
        }
    });
    let (budget, duration) = budget.admit(
        Field::optional(palette.grey, '\u{f051b}', duration(status.elapsed_ms)),
        reserve,
    );
    let cache = cache_text(status, &around.zone);
    let (budget, cache) = budget.admit(Field::optional(palette.grey, '\u{f162a}', cache), reserve);
    let branch = branch.and_then(|branch| truncate(branch, budget.room()));

    [
        Some(model),
        effort,
        root,
        branch,
        duration,
        cache,
        misses,
        Some(pie),
    ]
    .iter()
    .flatten()
    .map(Field::render)
    .collect::<Vec<_>>()
    .join(SEP)
}

fn truncate(field: Field, room: i64) -> Option<Field> {
    if len(&field.text) <= room {
        Some(field)
    } else if room > 1 {
        let kept = usize::try_from(room - 1).unwrap_or_default();
        let text = field
            .text
            .chars()
            .take(kept)
            .chain(['…'])
            .collect::<String>();
        Some(Field { text, ..field })
    } else {
        None
    }
}

fn len(text: &str) -> i64 {
    i64::try_from(text.chars().count()).unwrap_or(i64::MAX)
}

/// The pie filled to the nearest eighth, full only at 100%.
fn slice(percent: i64) -> usize {
    let eighth = (percent.min(100) * 8 + 50) / 100;
    let eighth = if eighth == 8 && percent < 100 {
        7
    } else {
        eighth
    };
    usize::try_from(eighth.clamp(0, 8)).unwrap_or_default()
}

fn context_text(context: &Context) -> String {
    format!(
        "{}/{} ({}%)",
        human(context.used),
        human(context.size),
        context.percent
    )
}

fn human(n: f64) -> String {
    if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 1000.0 {
        format!("{:.0}k", n / 1000.0)
    } else {
        format!("{}", n.trunc())
    }
}

/// Hours and minutes; nothing before the first second.
fn duration(elapsed_ms: i64) -> String {
    let seconds = elapsed_ms / 1000;
    if seconds > 0 {
        format!("{:02}:{:02}", seconds / 3600, seconds % 3600 / 60)
    } else {
        String::new()
    }
}

fn cache_text(status: &Status, zone: &TimeZone) -> String {
    if !status.cache.observed || status.cache.expires <= 0 {
        return String::new();
    }
    Timestamp::from_second(status.cache.expires)
        .map(|at| {
            at.to_zoned(zone.clone())
                .strftime("%d %b %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::Status;
    use serde_json::json;

    fn around(columns: i64, root: &str, branch: &str) -> Surroundings {
        Surroundings {
            appearance: Appearance::Dark,
            columns,
            root: root.into(),
            branch: branch.into(),
            effort: Effort::new("high"),
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

    #[test]
    fn human_scales_tokens() {
        assert_eq!(human(999.0), "999");
        assert_eq!(human(45_200.0), "45k");
        assert_eq!(human(1_250_000.0), "1.2M");
    }

    #[test]
    fn pie_is_full_only_at_one_hundred() {
        assert_eq!(slice(0), 0);
        assert_eq!(slice(50), 4);
        assert_eq!(slice(99), 7);
        assert_eq!(slice(100), 8);
    }

    #[test]
    fn duration_shows_hours_and_minutes() {
        assert_eq!(duration(999), "");
        assert_eq!(duration(3_725_000), "01:02");
    }

    #[test]
    fn narrow_lines_drop_fields_and_truncate_the_branch() {
        let status = Status::parse(&json!({
            "model": {"display_name": "Opus"},
            "context_window": {"total_input_tokens": 45_200, "used_percentage": 22},
            "cost": {"total_duration_ms": 3_725_000},
        }));
        let wide = strip(&render(
            &status,
            &around(120, "dotfiles", "feature/long-branch-name"),
        ));
        assert!(
            wide.contains("feature/long-branch-name") && wide.contains("01:02"),
            "{wide}"
        );
        let narrow = strip(&render(
            &status,
            &around(64, "dotfiles", "feature/long-branch-name"),
        ));
        assert!(
            narrow.contains('…') && !narrow.contains("01:02"),
            "{narrow}"
        );
        let narrowest = strip(&render(
            &status,
            &around(48, "dotfiles", "feature/long-branch-name"),
        ));
        assert!(!narrowest.contains("feature"), "{narrowest}");
    }
}
