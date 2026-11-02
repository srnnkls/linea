mod child;
mod config;
mod events;
mod format;
mod hooks;
mod host;
mod line;
mod sections;
mod session;
mod template;
mod workspace;

use std::{io::Read, path::Path};

use clap::{Parser, Subcommand};
use jiff::Timestamp;
use serde_json::Value;

use crate::{
    hooks::HookOutputs,
    host::{Pane, Terminal},
    sections::BuiltInTexts,
    session::Session,
    template::Scope,
    workspace::Workspace,
};

/// A status line for Claude Code: reads the session's status JSON on stdin
/// and prints one line fitted to the terminal.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Print the built-in configuration.
    Defaults,
    /// Deliver an event; started detached by linea itself.
    #[command(name = events::SUBCOMMAND, hide = true)]
    Deliver,
    /// Run a hook and cache its output; started detached by linea itself.
    #[command(name = hooks::SUBCOMMAND, hide = true)]
    Refresh,
}

fn main() {
    match Cli::parse().command {
        None => println!("{}", status_line(&stdin())),
        Some(Command::Defaults) => print!("{}", config::DEFAULTS),
        Some(Command::Deliver) => {
            if let Some(delivery) = child::receive() {
                events::deliver(&delivery);
            }
        }
        Some(Command::Refresh) => {
            if let Some(refresh) = child::receive() {
                hooks::refresh(&refresh);
            }
        }
    }
}

fn stdin() -> String {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    input
}

/// The effects first, each read once into a domain value; then the line as a
/// pure function of them.
fn status_line(raw: &str) -> String {
    let input: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
    let parsed = Session::parse(&input);
    let config = match config::load(Some(&host::global_config()), Path::new(&parsed.dir)) {
        Ok(config) => config,
        Err(error) => return format!("\x1b[31mlinea: {error}\x1b[0m"),
    };
    let session = parsed.or_effort(|| host::effort(&config.claude));
    let terminal = Terminal::observe(config.mode);
    let workspace = Workspace::locate(&session.dir);
    let pane = Pane::observe(&config.herdr);
    let outputs = HookOutputs::read(&config, &session, raw);

    let texts = BuiltInTexts::new(
        &config.built_ins,
        &session,
        &workspace,
        pane.as_ref(),
        &terminal.zone,
    );
    let scope = Scope::new(&input, &texts.for_templates(), host::vars(), &terminal.zone);
    let parts = sections::parts(&config, &texts, &outputs, &scope, terminal.appearance);

    events::send(
        &config.events,
        events::state(&config, &parts).as_deref(),
        &scope,
        &session.id,
        Timestamp::now(),
    );
    line::render(&parts, terminal.columns, &config.line)
}
