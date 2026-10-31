mod host;
mod line;
mod report;
mod status;

use std::io::Read;

use jiff::{Timestamp, tz::TimeZone};
use serde_json::Value;

use crate::{line::Surroundings, status::Status};

fn main() {
    let mut args = std::env::args_os().skip(1);
    if args.next().is_some_and(|arg| arg == report::RECEIPT) {
        report::receipt(args);
        return;
    }

    let mut raw = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut raw);
    let input: Value = serde_json::from_slice(&raw).unwrap_or(Value::Null);
    let status = Status::parse(&input);
    let effort = status.effort.clone().or_else(host::effort);

    report::model(
        &input,
        &status,
        effort.as_ref(),
        Timestamp::now().as_second(),
    );

    let (root, branch) = host::workspace(&status.dir);
    let around = Surroundings {
        appearance: host::appearance(),
        columns: host::columns(),
        root,
        branch,
        effort,
        zone: TimeZone::system(),
    };
    println!("{}", line::render(&status, &around));
}
