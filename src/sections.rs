//! What each section shows: a built-in text from the session, a template, or a
//! hook's output.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    config::{BuiltIn, Config, Icon, Source},
    format,
    line::Part,
    session::Session,
    template::Scope,
    world::World,
};

/// The configured sections that have something to show, in drawing order. A
/// template that renders empty, or fails to render, leaves its section out.
pub fn resolve(config: &Config, session: &Session, world: &World, input: &Value) -> Vec<Part> {
    let scope = config
        .sections
        .iter()
        .any(|section| match &section.source {
            Source::Template(_) => true,
            Source::Hook(hook) => hook.template.is_some(),
            Source::BuiltIn(_) => false,
        })
        .then(|| {
            let linea: BTreeMap<_, _> = config
                .built_ins
                .iter()
                .map(|built_in| {
                    (
                        built_in.name(),
                        text(built_in, session, world).unwrap_or_default(),
                    )
                })
                .collect();
            Scope::new(input, &linea, &world.vars, &world.zone)
        });
    let rendered = |template, output| {
        scope
            .as_ref()
            .and_then(|scope| scope.render(template, output).ok())
            .filter(|text| !text.is_empty())
    };

    config
        .sections
        .iter()
        .filter_map(|section| {
            let text = match &section.source {
                Source::BuiltIn(built_in) => text(built_in, session, world),
                Source::Template(template) => rendered(template, None),
                Source::Hook(hook) => {
                    let output = world.hooks.get(&section.name)?;
                    match &hook.template {
                        Some(template) => rendered(template, Some(output)),
                        None => Some(output.clone()),
                    }
                }
            }?;
            let icon = match &section.icon {
                Icon::Bare => String::new(),
                Icon::Fixed(icon) => icon.clone(),
                Icon::Steps(steps) => format::step(steps.len(), session.context.percent)
                    .and_then(|at| steps.get(at).cloned())
                    .unwrap_or_default(),
            };
            Some(Part {
                name: section.name.clone(),
                colour: section.shade.under(world.appearance),
                icon,
                text,
                fit: section.fit.clone(),
            })
        })
        .collect()
}

/// A built-in section's text; `None` leaves the section out.
fn text(built_in: &BuiltIn, session: &Session, world: &World) -> Option<String> {
    let present = |text: String| Some(text).filter(|text| !text.is_empty());
    match built_in {
        BuiltIn::Model => Some(session.model.name.clone()),
        BuiltIn::Effort => world
            .effort
            .as_ref()
            .map(|effort| effort.as_str().to_owned()),
        BuiltIn::Project => present(world.workspace.project.clone()),
        BuiltIn::Branch => present(world.workspace.branch.clone()),
        BuiltIn::Pane => world.pane.clone(),
        BuiltIn::Duration => present(format::elapsed(session.elapsed)),
        BuiltIn::Cache { format } => session
            .cache
            .expires
            .and_then(|at| present(format::time(at, format, &world.zone))),
        BuiltIn::CacheMisses => {
            (session.cache.misses > 0).then(|| session.cache.misses.to_string())
        }
        BuiltIn::Context => Some(format::context(&session.context)),
    }
}
