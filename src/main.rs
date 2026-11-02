mod child;
mod config;
mod format;
mod hooks;
mod host;
mod line;
mod report;
mod sections;
mod session;
mod template;
mod workspace;
mod world;

use std::{io::Read, path::Path};

use clap::{Parser, Subcommand};
use jiff::Timestamp;
use serde_json::Value;

use crate::{session::Session, world::World};

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
    /// Deliver a model and effort report; started detached by linea itself.
    #[command(name = report::SUBCOMMAND, hide = true)]
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
                report::deliver(&delivery);
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

/// Effects first, gathered into a `World`; then the line as a pure function of
/// the session, the configuration and that world.
fn status_line(raw: &str) -> String {
    let input: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
    let session = Session::parse(&input);
    let config = match config::load(Some(&host::global_config()), Path::new(&session.dir)) {
        Ok(config) => config,
        Err(error) => return format!("\x1b[31mlinea: {error}\x1b[0m"),
    };
    let world = World::observe(&config, &session, raw);
    report::send(
        &config.report,
        &input,
        &session,
        world.effort.as_ref(),
        Timestamp::now(),
    );
    let parts = sections::resolve(&config, &session, &world, &input);
    line::render(&parts, world.columns, &config.line)
}
