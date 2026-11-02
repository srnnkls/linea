//! Section templates: minijinja over Claude Code's status JSON, with the
//! built-in section texts under `linea`, the environment under `env` and a
//! hook's output as `output`.

use std::collections::BTreeMap;

use jiff::{Timestamp, tz::TimeZone};
use minijinja::{Environment, UndefinedBehavior, value::Serde};
use serde_json::{Map, Value};

use crate::sections;

pub struct Templates {
    env: Environment<'static>,
    base: Map<String, Value>,
}

impl Templates {
    pub fn new(input: &Value, linea: BTreeMap<String, String>, zone: TimeZone) -> Self {
        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Chainable);
        env.add_filter("tokens", |n: Option<f64>| {
            n.map(sections::human).unwrap_or_default()
        });
        env.add_filter("duration", |ms: Option<f64>| {
            ms.map(|ms| sections::duration(floor(ms)))
                .unwrap_or_default()
        });
        env.add_filter(
            "time",
            move |seconds: Option<f64>, format: Option<String>| {
                seconds
                    .and_then(|seconds| Timestamp::from_second(floor(seconds)).ok())
                    .map(|at| {
                        at.to_zoned(zone.clone())
                            .strftime(format.as_deref().unwrap_or("%H:%M"))
                            .to_string()
                    })
                    .unwrap_or_default()
            },
        );

        let mut base = input.as_object().cloned().unwrap_or_default();
        base.insert(
            "linea".into(),
            serde_json::to_value(linea).unwrap_or_default(),
        );
        base.insert(
            "env".into(),
            std::env::vars()
                .map(|(k, v)| (k, Value::String(v)))
                .collect::<Map<_, _>>()
                .into(),
        );
        Self { env, base }
    }

    /// The template rendered, trimmed; `output` is a hook's text, when the section has one.
    pub fn render(&self, source: &str, output: Option<&str>) -> Result<String, String> {
        let mut scope = self.base.clone();
        if let Some(output) = output {
            scope.insert("output".into(), Value::String(output.into()));
        }
        self.env
            .render_str(source, Serde(Value::Object(scope)))
            .map(|text| text.trim().to_owned())
            .map_err(|error| error.to_string())
    }
}

/// Whether a template parses.
pub fn check(source: &str) -> Result<(), String> {
    Environment::new()
        .template_from_str(source)
        .map(|_| ())
        .map_err(|error| error.to_string())
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
    use serde_json::json;

    fn templates() -> Templates {
        let input = json!({
            "cost": {"total_cost_usd": 1.23456, "total_duration_ms": 3_725_000},
            "context_window": {"total_output_tokens": 45_200},
            "rate_limits": {"five_hour": {"resets_at": 1_761_940_800}},
        });
        let linea = BTreeMap::from([("branch".to_owned(), "main".to_owned())]);
        Templates::new(&input, linea, TimeZone::UTC)
    }

    #[test]
    fn fields_filters_and_built_ins_render() {
        let t = templates();
        assert_eq!(
            t.render("${{ cost.total_cost_usd | round(2) }}", None)
                .unwrap(),
            "$1.23"
        );
        assert_eq!(
            t.render("{{ context_window.total_output_tokens | tokens }}", None)
                .unwrap(),
            "45k"
        );
        assert_eq!(
            t.render("{{ cost.total_duration_ms | duration }}", None)
                .unwrap(),
            "01:02"
        );
        assert_eq!(
            t.render("{{ rate_limits.five_hour.resets_at | time }}", None)
                .unwrap(),
            "20:00"
        );
        assert_eq!(t.render("{{ linea.branch }}", None).unwrap(), "main");
        assert_eq!(t.render("{{ output }}°", Some("12")).unwrap(), "12°");
    }

    #[test]
    fn linea_filters_render_a_missing_field_empty() {
        let t = templates();
        assert_eq!(
            t.render(
                "{{ nope | tokens }}{{ nope | duration }}{{ nope | time }}",
                None
            )
            .unwrap(),
            ""
        );
    }

    #[test]
    fn missing_fields_render_empty() {
        assert_eq!(templates().render("{{ pr.number }}", None).unwrap(), "");
        assert_eq!(
            templates()
                .render("{% if pr %}#{{ pr.number }}{% endif %}", None)
                .unwrap(),
            ""
        );
    }

    #[test]
    fn a_broken_template_is_caught() {
        assert!(check("{{ cost.").is_err());
        assert!(check("{{ cost.total_cost_usd }}").is_ok());
    }
}
