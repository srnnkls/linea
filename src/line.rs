//! Fitting sections into the terminal width and drawing them.

use std::collections::HashSet;

use crate::config::{Fit, LineStyle, Rgb};

const RESET: &str = "\x1b[0m";
/// A growing part holds columns at its rank only when it can show this many.
const HOLD_AT_LEAST: i64 = 4;

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
    fn width(&self, style: &LineStyle) -> i64 {
        self.lead(style) + len(&self.text)
    }

    /// Columns before the text: icon and gap, or none without an icon.
    fn lead(&self, style: &LineStyle) -> i64 {
        if self.icon.is_empty() {
            0
        } else {
            len(&self.icon) + len(&style.gap)
        }
    }

    fn draw(&self, style: &LineStyle) -> String {
        let Rgb(r, g, b) = self.colour;
        let gap = if self.icon.is_empty() { "" } else { &style.gap };
        format!(
            "\x1b[38;2;{r};{g};{b}m{}{gap}{}{RESET}",
            self.icon, self.text
        )
    }

    fn cut(&self, room: i64) -> Option<Self> {
        if len(&self.text) <= room {
            Some(self.clone())
        } else if room > 1 {
            let kept = usize::try_from(room - 1).unwrap_or_default();
            let text = self.text.chars().take(kept).chain(['…']).collect();
            Some(Self {
                text,
                ..self.clone()
            })
        } else {
            None
        }
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

    /// Text columns left for one more part.
    fn room(self, part: &Part, style: &LineStyle) -> i64 {
        self.columns - self.taken - self.separator - part.lead(style)
    }
}

/// Parts arrive in drawing order. `always` parts take their columns first;
/// ranked parts then claim theirs from the lowest rank up while they fit. A
/// growing part holds up to its `min` at its rank and, once every rank is
/// placed, takes whatever is left, cut short with an ellipsis.
pub fn render(parts: &[Part], columns: i64, style: &LineStyle) -> String {
    let separator = len(&style.separator);
    let always: Vec<&Part> = parts
        .iter()
        .filter(|part| part.fit == Fit::Always)
        .collect();
    let taken = always.iter().map(|part| part.width(style)).sum::<i64>()
        + separator * (i64::try_from(always.len()).unwrap_or(0) - 1).max(0);
    let mut budget = Budget {
        columns: columns - style.margin,
        taken,
        separator,
    };
    let mut shown: HashSet<&str> = always.iter().map(|part| part.name.as_str()).collect();

    let mut ranked: Vec<_> = parts
        .iter()
        .enumerate()
        .filter_map(|(at, part)| match &part.fit {
            Fit::Ranked(claim) => Some((claim.rank, at, part, claim)),
            Fit::Always => None,
        })
        .collect();
    ranked.sort_by_key(|&(rank, at, ..)| (rank, at));

    let mut reserve = 0;
    let mut growing = None;
    for (.., part, claim) in ranked {
        if claim
            .requires
            .as_deref()
            .is_some_and(|required| !shown.contains(required))
        {
            continue;
        }
        if claim.grow {
            let held = budget.room(part, style).min(claim.min).min(len(&part.text));
            if held >= HOLD_AT_LEAST {
                reserve = separator + part.lead(style) + held;
            }
            growing = Some(part);
            shown.insert(&part.name);
        } else if budget.fits(part.width(style), reserve) {
            budget = budget.take(part.width(style));
            shown.insert(&part.name);
        }
    }
    let grown = growing.and_then(|part| part.cut(budget.room(part, style)));

    parts
        .iter()
        .filter_map(|part| {
            if growing.is_some_and(|growing| growing.name == part.name) {
                grown.clone()
            } else {
                shown.contains(part.name.as_str()).then(|| part.clone())
            }
        })
        .map(|part| part.draw(style))
        .collect::<Vec<_>>()
        .join(&style.separator)
}

fn len(text: &str) -> i64 {
    i64::try_from(text.chars().count()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Claim;

    fn style() -> LineStyle {
        LineStyle {
            separator: "  ".into(),
            gap: " ".into(),
            margin: 0,
        }
    }

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

    fn draw(parts: &[Part], columns: i64) -> String {
        let line = render(parts, columns, &style());
        let mut plain = String::new();
        let mut chars = line.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                chars.by_ref().find(|&c| c == 'm');
            } else {
                plain.push(c);
            }
        }
        plain
    }

    #[test]
    fn lower_ranks_claim_columns_first_and_keep_drawing_order() {
        let parts = [
            part("a", "aaaa", Fit::Always),
            part("b", "bbbb", ranked(2)),
            part("c", "cccc", ranked(1)),
        ];
        assert_eq!(draw(&parts, 40), "# aaaa  # bbbb  # cccc");
        assert_eq!(draw(&parts, 6 + 8), "# aaaa  # cccc");
    }

    #[test]
    fn a_growing_part_holds_its_min_then_takes_what_is_left() {
        let grow = Fit::Ranked(Claim {
            rank: 1,
            grow: true,
            min: 4,
            requires: None,
        });
        let parts = [
            part("a", "aaaa", Fit::Always),
            part("g", "a-long-branch", grow),
            part("z", "zz", ranked(2)),
        ];
        assert_eq!(draw(&parts, 60), "# aaaa  # a-long-branch  # zz");
        assert_eq!(draw(&parts, 24), "# aaaa  # a-long-…  # zz");
        assert_eq!(draw(&parts, 18), "# aaaa  # a-long-…");
    }

    #[test]
    fn a_part_is_drawn_only_with_what_it_requires() {
        let needs = Fit::Ranked(Claim {
            rank: 2,
            grow: false,
            min: 0,
            requires: Some("p".into()),
        });
        let parts = [
            part("a", "aaaa", Fit::Always),
            part("p", "pppppppppp", ranked(1)),
            part("n", "n", needs),
        ];
        assert_eq!(draw(&parts, 12), "# aaaa");
        assert_eq!(draw(&parts, 40), "# aaaa  # pppppppppp  # n");
    }
}
