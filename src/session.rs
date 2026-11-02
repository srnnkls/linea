//! The session Claude Code reports on stdin, parsed once at the boundary.
//! Fields read with jq's `//` semantics: null and false count as absent.

use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub id: SessionId,
    pub model: Model,
    /// The working directory, as Claude Code reports it.
    pub dir: String,
    pub context: ContextUse,
    pub elapsed: SignedDuration,
    pub effort: Option<Effort>,
    pub cache: PromptCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextUse {
    pub used: Tokens,
    pub size: Tokens,
    pub percent: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens(pub f64);

/// A reasoning effort level; never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effort(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptCache {
    /// When the cache goes cold, once Claude Code has seen it warm.
    pub expires: Option<Timestamp>,
    pub misses: u64,
}

impl SessionId {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The id as a file name: anything but ASCII letters, digits and dashes becomes `_`.
    pub fn file_name(&self) -> String {
        let name: String = self
            .0
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if name.is_empty() { "none".into() } else { name }
    }
}

impl Effort {
    pub fn new(level: impl Into<String>) -> Option<Self> {
        let level = level.into();
        (!level.is_empty()).then_some(Self(level))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Session {
    /// The session with a configured effort level where it reports none.
    #[must_use]
    pub fn or_effort(self, configured: impl FnOnce() -> Option<Effort>) -> Self {
        let effort = self.effort.or_else(configured);
        Self { effort, ..self }
    }

    pub fn parse(input: &Value) -> Self {
        let effort = match present(input, &["effort"]) {
            Some(Value::Object(effort)) => effort.get("level").filter(|v| is_present(v)),
            other => other,
        }
        .or_else(|| present(input, &["effort_level"]))
        .map(text)
        .and_then(Effort::new);

        let observed =
            present(input, &["prompt_cache", "caching_observed"]) == Some(&Value::Bool(true));
        let expires = integer(input, &["prompt_cache", "expires_at"]);

        Self {
            id: SessionId(string(input, &["session_id"])),
            model: Model {
                name: present(input, &["model", "display_name"])
                    .map_or_else(|| "Claude".into(), text),
                id: string(input, &["model", "id"]),
            },
            dir: present(input, &["workspace", "current_dir"])
                .or_else(|| present(input, &["cwd"]))
                .map_or_else(|| ".".into(), text),
            context: ContextUse {
                used: Tokens(
                    number(input, &["context_window", "total_input_tokens"]).unwrap_or(0.0),
                ),
                size: Tokens(
                    number(input, &["context_window", "context_window_size"]).unwrap_or(200_000.0),
                ),
                percent: floor(input, &["context_window", "used_percentage"]),
            },
            elapsed: SignedDuration::from_millis(floor(input, &["cost", "total_duration_ms"])),
            effort,
            cache: PromptCache {
                expires: (observed && expires > 0)
                    .then(|| Timestamp::from_second(expires).ok())
                    .flatten(),
                misses: u64::try_from(integer(input, &["prompt_cache", "misses"])).unwrap_or(0),
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
        let session = Session::parse(&json!({}));
        assert_eq!(session.model.name, "Claude");
        assert_eq!(session.dir, ".");
        assert!((session.context.size.0 - 200_000.0).abs() < f64::EPSILON);
        assert_eq!(session.effort, None);
        assert_eq!(session.cache.expires, None);
    }

    #[test]
    fn effort_reads_an_object_level_or_a_bare_string() {
        let level = |input| Session::parse(&input).effort.map(|e| e.as_str().to_owned());
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
        let session = Session::parse(&json!({"cwd": "/a", "workspace": {"current_dir": null}}));
        assert_eq!(session.dir, "/a");
    }

    #[test]
    fn percent_floors() {
        let session = Session::parse(&json!({"context_window": {"used_percentage": 41.9}}));
        assert_eq!(session.context.percent, 41);
    }

    #[test]
    fn the_cache_expires_only_once_observed() {
        let cache = |observed| {
            Session::parse(
                &json!({"prompt_cache": {"caching_observed": observed, "expires_at": 100}}),
            )
            .cache
        };
        assert_eq!(cache(false).expires, None);
        assert_eq!(cache(true).expires, Timestamp::from_second(100).ok());
    }

    #[test]
    fn a_session_id_becomes_a_safe_file_name() {
        let id = |id: &str| Session::parse(&json!({"session_id": id})).id.file_name();
        assert_eq!(id("a1b2-c3"), "a1b2-c3");
        assert_eq!(id("../x"), "___x");
        assert_eq!(id(""), "none");
    }
}
