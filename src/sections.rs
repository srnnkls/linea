//! What each section shows: a built-in text from the session, a template, or a
//! hook's output, under an icon that is fixed or picked by a state.

use std::collections::BTreeMap;

use jiff::tz::TimeZone;

use crate::{
    config::{Appearance, BuiltIn, Config, Icon, Source},
    format,
    hooks::HookOutputs,
    host::Pane,
    line::Part,
    session::Session,
    template::Scope,
    workspace::Workspace,
};

/// Each built-in section's text, by name; absent where it has nothing to show.
#[derive(Debug, Clone, Default)]
pub struct BuiltInTexts(BTreeMap<&'static str, String>);

impl BuiltInTexts {
    pub fn new(
        built_ins: &[BuiltIn],
        session: &Session,
        workspace: &Workspace,
        pane: Option<&Pane>,
        zone: &TimeZone,
    ) -> Self {
        Self(
            built_ins
                .iter()
                .filter_map(|built_in| {
                    text(built_in, session, workspace, pane, zone)
                        .map(|text| (built_in.name(), text))
                })
                .collect(),
        )
    }

    fn get(&self, built_in: &BuiltIn) -> Option<&str> {
        self.0.get(built_in.name()).map(String::as_str)
    }

    /// Every built-in's text for templates, an absent one as empty.
    pub fn for_templates(&self) -> BTreeMap<&'static str, String> {
        BuiltIn::NAMES
            .iter()
            .map(|name| (*name, self.0.get(name).cloned().unwrap_or_default()))
            .collect()
    }
}

/// The configured sections that have something to show, in drawing order. A
/// template that renders empty, or fails to render, leaves its section out.
pub fn parts(
    config: &Config,
    texts: &BuiltInTexts,
    outputs: &HookOutputs,
    scope: &Scope,
    appearance: Appearance,
) -> Vec<Part> {
    let rendered = |template, output| {
        scope
            .render(template, output)
            .ok()
            .filter(|text| !text.is_empty())
    };
    config
        .sections
        .iter()
        .filter_map(|section| {
            let text = match &section.source {
                Source::BuiltIn(built_in) => texts.get(built_in).map(str::to_owned),
                Source::Template(template) => rendered(template, None),
                Source::Hook(hook) => {
                    let output = outputs.get(&section.name)?;
                    match &hook.template {
                        Some(template) => rendered(template, Some(output)),
                        None => Some(output.to_owned()),
                    }
                }
            }?;
            let icon = match &section.icon {
                Icon::Bare => "",
                Icon::Fixed(icon) => icon,
                Icon::Mapped(map) => map.pick(scope.lookup(&map.state)).unwrap_or_default(),
            };
            Some(Part {
                name: section.name.clone(),
                colour: section.shade.under(appearance),
                icon: icon.to_owned(),
                text,
                fit: section.fit.clone(),
            })
        })
        .collect()
}

fn text(
    built_in: &BuiltIn,
    session: &Session,
    workspace: &Workspace,
    pane: Option<&Pane>,
    zone: &TimeZone,
) -> Option<String> {
    let present = |text: String| Some(text).filter(|text| !text.is_empty());
    match built_in {
        BuiltIn::Model => Some(session.model.name.clone()),
        BuiltIn::Effort => session
            .effort
            .as_ref()
            .map(|effort| effort.as_str().to_owned()),
        BuiltIn::Project => present(workspace.project.clone()),
        BuiltIn::Branch => present(workspace.branch.clone()),
        BuiltIn::Pane => pane.map(|Pane(pane)| pane.clone()),
        BuiltIn::Duration => present(format::elapsed(session.elapsed)),
        BuiltIn::Cache { format } => session
            .cache
            .expires
            .and_then(|at| present(format::time(at, format, zone))),
        BuiltIn::CacheMisses => {
            (session.cache.misses > 0).then(|| session.cache.misses.to_string())
        }
        BuiltIn::Context => Some(format::context(&session.context)),
    }
}
