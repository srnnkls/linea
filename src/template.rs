//! Section templates: minijinja over Claude Code's status JSON, with the
//! built-in section texts under `linea`, the environment under `env` and a
//! hook's output as `output`.

use std::collections::BTreeMap;

use jiff::{SignedDuration, Timestamp, tz::TimeZone};
use minijinja::{Environment, UndefinedBehavior, value::Serde};
use serde_json::{Map, Value};

use crate::{format, session::Tokens};

/// A template source that parses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template(String);

/// Everything templates and icon states read: built once per render, then
/// shared by every section and the events body.
pub struct Scope {
    env: Environment<'static>,
    base: Map<String, Value>,
}

impl Template {
    pub fn parse(source: &str) -> Result<Self, String> {
        Environment::new()
            .template_from_str(source)
            .map(|_| Self(source.to_owned()))
            .map_err(|error| error.to_string())
    }
}

impl Scope {
    pub fn new(
        input: &Value,
        linea: &BTreeMap<&'static str, String>,
        vars: BTreeMap<String, String>,
        zone: &TimeZone,
    ) -> Self {
        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Chainable);
        env.add_filter("tokens", |n: Option<f64>| {
            n.map(|n| format::tokens(Tokens(n))).unwrap_or_default()
        });
        env.add_filter("duration", |ms: Option<f64>| {
            ms.map(|ms| format::elapsed(SignedDuration::from_millis(floor(ms))))
                .unwrap_or_default()
        });
        let zone = zone.clone();
        env.add_filter(
            "time",
            move |seconds: Option<f64>, pattern: Option<String>| {
                seconds
                    .and_then(|seconds| Timestamp::from_second(floor(seconds)).ok())
                    .map(|at| format::time(at, pattern.as_deref().unwrap_or("%H:%M"), &zone))
                    .unwrap_or_default()
            },
        );

        let mut base = input.as_object().cloned().unwrap_or_default();
        base.insert(
            "linea".into(),
            linea
                .iter()
                .map(|(k, v)| ((*k).to_owned(), Value::from(v.as_str())))
                .collect::<Map<_, _>>()
                .into(),
        );
        base.insert(
            "env".into(),
            vars.into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect::<Map<_, _>>()
                .into(),
        );
        Self { env, base }
    }

    /// The value at a dotted path, such as `context_window.used_percentage` or `linea.effort`.
    pub fn lookup(&self, path: &str) -> Option<&Value> {
        let mut keys = path.split('.');
        let first = self.base.get(keys.next()?)?;
        keys.try_fold(first, |value, key| value.get(key))
            .filter(|value| !value.is_null())
    }

    /// The template rendered and trimmed; `output` is a hook's text, when the section has one.
    pub fn render(
        &self,
        template: &Template,
        output: Option<&str>,
    ) -> Result<String, minijinja::Error> {
        let mut scope = self.base.clone();
        if let Some(output) = output {
            scope.insert("output".into(), Value::from(output));
        }
        self.env
            .render_str(&template.0, Serde(Value::Object(scope)))
            .map(|text| text.trim().to_owned())
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a saturating floor of a JSON number"
)]
fn floor(n: f64) -> i64 {
    n.floor() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linea_filters_render_a_missing_field_empty() {
        let scope = Scope::new(
            &Value::Null,
            &BTreeMap::new(),
            BTreeMap::new(),
            &TimeZone::UTC,
        );
        let template =
            Template::parse("{{ nope | tokens }}{{ nope | duration }}{{ nope | time }}").unwrap();
        assert_eq!(scope.render(&template, None).unwrap(), "");
    }
}
