//! How numbers, durations and times read on the line.

use jiff::{SignedDuration, Timestamp, tz::TimeZone};

use crate::session::{ContextUse, Tokens};

/// `999`, `45k`, `1.2M`.
pub fn tokens(Tokens(n): Tokens) -> String {
    if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 1000.0 {
        format!("{:.0}k", n / 1000.0)
    } else {
        format!("{}", n.trunc())
    }
}

/// Hours and minutes; nothing before the first second.
pub fn elapsed(elapsed: SignedDuration) -> String {
    let seconds = elapsed.as_secs();
    if seconds > 0 {
        format!("{:02}:{:02}", seconds / 3600, seconds % 3600 / 60)
    } else {
        String::new()
    }
}

pub fn time(at: Timestamp, pattern: &str, zone: &TimeZone) -> String {
    at.to_zoned(zone.clone()).strftime(pattern).to_string()
}

/// `45k/200k (22%)`.
pub fn context(context: &ContextUse) -> String {
    format!(
        "{}/{} ({}%)",
        tokens(context.used),
        tokens(context.size),
        context.percent
    )
}

/// The step for a share: empty to full, the last one only at 100%.
pub fn step(steps: usize, percent: i64) -> Option<usize> {
    let last = i64::try_from(steps).ok()? - 1;
    if last < 0 {
        return None;
    }
    let at = (percent.min(100) * last + 50) / 100;
    let at = if at == last && percent < 100 {
        last - 1
    } else {
        at
    };
    usize::try_from(at.clamp(0, last)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_scale() {
        assert_eq!(tokens(Tokens(999.0)), "999");
        assert_eq!(tokens(Tokens(45_200.0)), "45k");
        assert_eq!(tokens(Tokens(1_250_000.0)), "1.2M");
    }
}
