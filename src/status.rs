//! The session state Claude Code pipes in, read with jq's `//` semantics:
//! a null or false value counts as absent.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub model: Model,
    pub session: String,
    pub dir: String,
    pub context: Context,
    pub elapsed_ms: i64,
    pub effort: Option<Effort>,
    pub cache: Cache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Context {
    pub used: f64,
    pub size: f64,
    pub percent: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effort(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cache {
    pub observed: bool,
    pub expires: i64,
    pub misses: i64,
}

impl Effort {
    /// An empty level is no level.
    pub fn new(level: impl Into<String>) -> Option<Self> {
        let level = level.into();
        (!level.is_empty()).then_some(Self(level))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The level limen hears: the implicit default is no explicit level.
    pub fn explicit(&self) -> &str {
        if self.0 == "default" { "" } else { &self.0 }
    }
}

impl Status {
    pub fn parse(input: &Value) -> Self {
        let effort = match present(input, &["effort"]) {
            Some(Value::Object(effort)) => effort.get("level").filter(|v| is_present(v)),
            other => other,
        }
        .or_else(|| present(input, &["effort_level"]))
        .map(text)
        .and_then(Effort::new);

        Self {
            model: Model {
                name: present(input, &["model", "display_name"])
                    .map_or_else(|| "Claude".into(), text),
                id: string(input, &["model", "id"]),
            },
            session: string(input, &["session_id"]),
            dir: present(input, &["workspace", "current_dir"])
                .or_else(|| present(input, &["cwd"]))
                .map_or_else(|| ".".into(), text),
            context: Context {
                used: number(input, &["context_window", "total_input_tokens"]).unwrap_or(0.0),
                size: number(input, &["context_window", "context_window_size"])
                    .unwrap_or(200_000.0),
                percent: floor(input, &["context_window", "used_percentage"]),
            },
            elapsed_ms: floor(input, &["cost", "total_duration_ms"]),
            effort,
            cache: Cache {
                observed: present(input, &["prompt_cache", "caching_observed"])
                    == Some(&Value::Bool(true)),
                expires: integer(input, &["prompt_cache", "expires_at"]),
                misses: integer(input, &["prompt_cache", "misses"]),
            },
        }
    }
}

fn is_present(value: &Value) -> bool {
    !matches!(value, Value::Null | Value::Bool(false))
}

fn present<'a>(input: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(input, |value, key| value.get(key))
        .filter(|value| is_present(value))
}

/// jq's `tostring`: strings as they are, anything else as JSON.
fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn string(input: &Value, path: &[&str]) -> String {
    present(input, path).map(text).unwrap_or_default()
}

fn number(input: &Value, path: &[&str]) -> Option<f64> {
    present(input, path).and_then(Value::as_f64)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a saturating floor, as jq's floor feeds shell arithmetic"
)]
fn floor(input: &Value, path: &[&str]) -> i64 {
    number(input, path).map_or(0, |n| n.floor() as i64)
}

/// A whole number; anything else reads as zero, as shell arithmetic rejects it.
fn integer(input: &Value, path: &[&str]) -> i64 {
    present(input, path).and_then(Value::as_i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_fill_an_empty_input() {
        let status = Status::parse(&json!({}));
        assert_eq!(status.model.name, "Claude");
        assert_eq!(status.dir, ".");
        assert!((status.context.size - 200_000.0).abs() < f64::EPSILON);
        assert_eq!(status.effort, None);
    }

    #[test]
    fn effort_reads_an_object_level_or_a_bare_string() {
        let level = |input| Status::parse(&input).effort.map(|e| e.as_str().to_owned());
        assert_eq!(
            level(json!({"effort": {"level": "high"}})),
            Some("high".into())
        );
        assert_eq!(level(json!({"effort": "low"})), Some("low".into()));
        assert_eq!(
            level(json!({"effort": {}, "effort_level": "max"})),
            Some("max".into())
        );
    }

    #[test]
    fn cwd_backs_up_a_missing_workspace() {
        assert_eq!(
            Status::parse(&json!({"cwd": "/a", "workspace": {"current_dir": null}})).dir,
            "/a"
        );
    }

    #[test]
    fn percent_floors() {
        assert_eq!(
            Status::parse(&json!({"context_window": {"used_percentage": 41.9}}))
                .context
                .percent,
            41
        );
    }
}
