mod config;
mod hooks;
mod host;
mod line;
mod report;
mod sections;
mod status;
mod template;

use std::{io::Read, path::Path};

use jiff::{Timestamp, tz::TimeZone};
use serde_json::Value;

use crate::{
    config::Config,
    hooks::{Hook, Session},
    sections::Surroundings,
    status::Status,
};

fn main() {
    let mut args = std::env::args_os().skip(1);
    match args.next() {
        Some(arg) if arg == report::RECEIPT => return report::receipt(args),
        Some(arg) if arg == hooks::HOOK => return hooks::refresh(args),
        Some(arg) if arg == "defaults" => return print!("{}", config::DEFAULTS),
        _ => {}
    }

    let mut raw = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut raw);
    let input: Value = serde_json::from_slice(&raw).unwrap_or(Value::Null);
    let status = Status::parse(&input);
    let config = match Config::load(Some(&host::linea_config()), Path::new(&status.dir)) {
        Ok(config) => config,
        Err(error) => return println!("\x1b[31mlinea: {error}\x1b[0m"),
    };
    let effort = status
        .effort
        .clone()
        .or_else(|| host::effort(&config.claude));

    report::model(
        &config.report,
        &input,
        &status,
        effort.as_ref(),
        Timestamp::now().as_second(),
    );

    let (root, branch) = host::workspace(&status.dir);
    let around = Surroundings {
        appearance: host::appearance(config.appearance.mode),
        root,
        branch,
        effort,
        pane: host::herdr_pane(&config.herdr),
        zone: TimeZone::system(),
    };
    let session = Session {
        id: &status.session,
        input: &raw,
        dir: &status.dir,
    };
    let parts = sections::parts(&config, &status, &around, &input, |name, section| {
        let run = section.run.as_deref()?;
        hooks::text(
            &Hook {
                name,
                run,
                refresh: section.refresh,
                timeout: section.timeout,
            },
            &session,
        )
    });
    println!("{}", line::render(&parts, host::columns(), &config.line));
}
