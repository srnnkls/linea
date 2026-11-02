//! Everything a render reads from outside the session: gathered once, up front,
//! so drawing the line is a pure function of the session, the configuration
//! and this.

use std::collections::BTreeMap;

use jiff::tz::TimeZone;

use crate::{
    config::{Appearance, Config, Source},
    hooks, host,
    session::{Effort, Session},
    workspace::Workspace,
};

#[derive(Debug, Clone)]
pub struct World {
    pub appearance: Appearance,
    pub columns: i64,
    pub workspace: Workspace,
    pub pane: Option<String>,
    /// The session's effort level, else the configured one.
    pub effort: Option<Effort>,
    pub zone: TimeZone,
    pub vars: BTreeMap<String, String>,
    /// Each hook section's last output, by section name.
    pub hooks: BTreeMap<String, String>,
}

impl World {
    pub fn observe(config: &Config, session: &Session, input: &str) -> Self {
        let hooks = config
            .sections
            .iter()
            .filter_map(|section| match &section.source {
                Source::Hook(hook) => {
                    hooks::cached(&section.name, hook, &session.id, &session.dir, input)
                        .map(|output| (section.name.clone(), output))
                }
                _ => None,
            })
            .collect();
        Self {
            appearance: host::appearance(config.mode),
            columns: host::columns(),
            workspace: Workspace::locate(&session.dir),
            pane: host::herdr_pane(&config.herdr),
            effort: session
                .effort
                .clone()
                .or_else(|| host::effort(&config.claude)),
            zone: TimeZone::system(),
            vars: std::env::vars().collect(),
            hooks,
        }
    }
}
