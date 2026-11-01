//! Fitting sections into the terminal width and drawing them.

use std::collections::HashSet;

use crate::config::{Fit, Line, Rgb};

const RESET: &str = "\x1b[0m";
/// A growing section holds columns at its rank only when it can show this many.
const HOLD_AT_LEAST: i64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

impl Appearance {
    /// Dark only when the interface style says so.
    pub fn from_style(style: Option<&str>) -> Self {
        if style == Some("Dark") {
            Self::Dark
        } else {
            Self::Light
        }
    }
}

/// A section with something to show, ready to be fitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub colour: Rgb,
    pub icon: String,
    pub text: String,
    pub fit: Fit,
}

impl Part {
    /// Columns on screen: icon, gap and text.
    fn width(&self, line: &Line) -> i64 {
        if self.icon.is_empty() {
            len(&self.text)
        } else {
            len(&self.icon) + len(&line.gap) + len(&self.text)
        }
    }

    fn render(&self, line: &Line) -> String {
        let Rgb(r, g, b) = self.colour;
        let gap = if self.icon.is_empty() { "" } else { &line.gap };
        format!(
            "\x1b[38;2;{r};{g};{b}m{}{gap}{}{RESET}",
            self.icon, self.text
        )
    }
}

/// Columns taken so far against the columns available.
#[derive(Debug, Clone, Copy)]
struct Budget {
    columns: i64,
    taken: i64,
    separator: i64,
}

impl Budget {
    fn fits(self, width: i64, reserve: i64) -> bool {
        self.taken + self.separator + width + reserve <= self.columns
    }

    fn take(self, width: i64) -> Self {
        Self {
            taken: self.taken + self.separator + width,
            ..self
        }
    }

    /// Text columns left for one more part with this icon.
    fn room(self, part: &Part, line: &Line) -> i64 {
        self.columns - self.taken - self.separator - (part.width(line) - len(&part.text))
    }
}

/// Parts arrive in drawing order. `always` parts take their columns first;
/// ranked parts then claim theirs from the lowest rank up while they fit. A
/// growing part holds up to its `min` at its rank and, once every rank is
/// placed, takes whatever is left, cut short with an ellipsis.
pub fn render(parts: &[Part], columns: i64, line: &Line) -> String {
    let columns = columns - line.margin;
    let always: Vec<&Part> = parts
        .iter()
        .filter(|part| part.fit == Fit::Always)
        .collect();
    let taken = always.iter().map(|part| part.width(line)).sum::<i64>()
        + len(&line.separator) * (i64::try_from(always.len()).unwrap_or(0) - 1).max(0);
    let mut budget = Budget {
        columns,
        taken,
        separator: len(&line.separator),
    };
    let mut shown: HashSet<&str> = always.iter().map(|part| part.name.as_str()).collect();

    let mut ranked: Vec<(i64, usize, &Part)> = parts
        .iter()
        .enumerate()
        .filter_map(|(at, part)| match &part.fit {
            Fit::Ranked(claim) => Some((claim.rank, at, part)),
            Fit::Always => None,
        })
        .collect();
    ranked.sort_by_key(|&(rank, at, _)| (rank, at));

    let mut reserve = 0;
    let mut growing: Option<&Part> = None;
    for (_, _, part) in ranked {
        let Fit::Ranked(claim) = &part.fit else {
            continue;
        };
        if claim
            .requires
            .as_deref()
            .is_some_and(|required| !shown.contains(required))
        {
            continue;
        }
        if claim.grow {
            let held = budget.room(part, line).min(claim.min).min(len(&part.text));
            if held >= HOLD_AT_LEAST {
                reserve = len(&line.separator) + (part.width(line) - len(&part.text)) + held;
            }
            growing = Some(part);
            shown.insert(&part.name);
        } else if budget.fits(part.width(line), reserve) {
            budget = budget.take(part.width(line));
            shown.insert(&part.name);
        }
    }
    let grown = growing.and_then(|part| truncate(part, budget.room(part, line)));

    parts
        .iter()
        .filter_map(|part| match &grown {
            Some(grown) if grown.name == part.name => Some(grown.clone()),
            _ if growing.is_some_and(|growing| growing.name == part.name) => None,
            _ if part.fit == Fit::Always || shown.contains(part.name.as_str()) => {
                Some(part.clone())
            }
            _ => None,
        })
        .map(|part| part.render(line))
        .collect::<Vec<_>>()
        .join(&line.separator)
}

fn truncate(part: &Part, room: i64) -> Option<Part> {
    if len(&part.text) <= room {
        Some(part.clone())
    } else if room > 1 {
        let kept = usize::try_from(room - 1).unwrap_or_default();
        let text = part.text.chars().take(kept).chain(['…']).collect();
        Some(Part {
            text,
            ..part.clone()
        })
    } else {
        None
    }
}

fn len(text: &str) -> i64 {
    i64::try_from(text.chars().count()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Claim, Config};

    fn part(name: &str, text: &str, fit: Fit) -> Part {
        Part {
            name: name.into(),
            colour: Rgb(1, 2, 3),
            icon: "#".into(),
            text: text.into(),
            fit,
        }
    }

    fn ranked(rank: i64) -> Fit {
        Fit::Ranked(Claim {
            rank,
            grow: false,
            min: 0,
            requires: None,
        })
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

    fn draw(parts: &[Part], columns: i64) -> String {
        strip(&render(parts, columns, &Config::defaults().line))
    }

    #[test]
    fn appearance_is_dark_only_by_name() {
        assert_eq!(Appearance::from_style(Some("Dark")), Appearance::Dark);
        assert_eq!(Appearance::from_style(Some("Light")), Appearance::Light);
        assert_eq!(Appearance::from_style(None), Appearance::Light);
    }

    #[test]
    fn lower_ranks_claim_columns_first_and_keep_drawing_order() {
        let parts = vec![
            part("a", "aaaa", Fit::Always),
            part("b", "bbbb", ranked(2)),
            part("c", "cccc", ranked(1)),
        ];
        assert_eq!(draw(&parts, 40), "# aaaa  # bbbb  # cccc");
        assert_eq!(draw(&parts, 4 + 6 + 8), "# aaaa  # cccc");
    }

    #[test]
    fn a_growing_part_takes_what_is_left() {
        let grow = Fit::Ranked(Claim {
            rank: 1,
            grow: true,
            min: 4,
            requires: None,
        });
        let parts = vec![
            part("a", "aaaa", Fit::Always),
            part("g", "a-long-branch", grow),
            part("z", "zz", ranked(2)),
        ];
        assert_eq!(draw(&parts, 60), "# aaaa  # a-long-branch  # zz");
        assert_eq!(draw(&parts, 4 + 24), "# aaaa  # a-long-…  # zz");
    }

    #[test]
    fn a_part_needs_what_it_requires() {
        let needs = Fit::Ranked(Claim {
            rank: 2,
            grow: false,
            min: 0,
            requires: Some("p".into()),
        });
        let parts = vec![
            part("a", "aaaa", Fit::Always),
            part("p", "pppppppppp", ranked(1)),
            part("n", "n", needs),
        ];
        assert_eq!(draw(&parts, 4 + 12), "# aaaa");
    }

    #[test]
    fn an_iconless_part_has_no_gap() {
        let mut bare = part("a", "text", Fit::Always);
        bare.icon.clear();
        assert_eq!(draw(&[bare], 40), "text");
    }
}
