//! Sends model and effort changes to a report command (limen by default, which
//! relays them to the herdr pane: the hook events it hears carry no effort).
//! One record per session holds the last key sent and whether the command took
//! it; an unanswered send is retried once `retry` has passed.

use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    child, config, host,
    session::{Effort, Session},
};

pub const SUBCOMMAND: &str = "deliver";

/// What a detached child needs to deliver one report.
#[derive(Debug, Serialize, Deserialize)]
pub struct Delivery {
    pub record: PathBuf,
    pub key: String,
    pub command: Vec<String>,
    pub received: String,
    pub event: Event,
}

/// A `StatusLine` hook event, as the report command reads it on stdin.
#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    hook_event_name: String,
    model: String,
    effort: String,
    session_id: Value,
    transcript_path: Value,
    cwd: Value,
}

#[derive(Debug, PartialEq, Eq)]
enum Record {
    Taken,
    Sent { at: i64 },
    Other,
}

impl Record {
    fn read(seen: &str, key: &str) -> Self {
        let seen = seen.trim_end_matches('\n');
        if seen == format!("{key} ok") {
            Self::Taken
        } else if let Some(rest) = seen.strip_prefix(&format!("{key} sent ")) {
            let at = rest
                .rsplit(' ')
                .next()
                .and_then(|at| at.parse().ok())
                .unwrap_or(0);
            Self::Sent { at }
        } else {
            Self::Other
        }
    }

    fn due(&self, now: i64, retry: i64) -> bool {
        match self {
            Self::Taken => false,
            Self::Sent { at } => now - at >= retry,
            Self::Other => true,
        }
    }
}

/// Sends the session's model and effort when they changed since the command last took them.
pub fn send(
    report: &config::Report,
    input: &Value,
    session: &Session,
    effort: Option<&Effort>,
    now: Timestamp,
) {
    let Some(program) = report.command.first() else {
        return;
    };
    if !report.enabled
        || report
            .requires
            .as_deref()
            .is_some_and(|name| host::var(name).is_none())
        || session.model.id.is_empty()
        || session.id.is_empty()
        || !host::on_path(program)
    {
        return;
    }
    let level = effort.map_or("", Effort::explicit);
    let key = format!("{}/{level}", session.model.id);
    let record = PathBuf::from(format!(
        "{}/claude-statusline-model.{}",
        host::temp_dir(),
        session.id.as_str()
    ));
    let seen = fs::read_to_string(&record).unwrap_or_default();
    let now = now.as_second();
    if !Record::read(&seen, &key).due(now, report.retry.as_secs())
        || fs::write(&record, format!("{key} sent {now}\n")).is_err()
    {
        return;
    }
    let field = |name| input.get(name).cloned().unwrap_or(Value::Null);
    let event = Event {
        hook_event_name: "StatusLine".into(),
        model: session.model.id.clone(),
        effort: level.into(),
        session_id: field("session_id"),
        transcript_path: field("transcript_path"),
        cwd: field("cwd"),
    };
    let delivery = Delivery {
        record,
        key,
        command: report.command.clone(),
        received: report.received.clone(),
        event,
    };
    let _ = child::detach(SUBCOMMAND, &delivery);
}

/// The detached child: hands the event to the command and records whether it was taken.
pub fn deliver(delivery: &Delivery) {
    let Some((program, args)) = delivery.command.split_first() else {
        return;
    };
    let Ok(mut command) = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return;
    };
    if let Some(mut stdin) = command.stdin.take()
        && let Ok(mut event) = serde_json::to_vec(&delivery.event)
    {
        event.push(b'\n');
        let _ = stdin.write_all(&event);
    }
    let Ok(output) = command.wait_with_output() else {
        return;
    };
    if String::from_utf8_lossy(&output.stdout).contains(&delivery.received) {
        let _ = fs::write(&delivery.record, format!("{} ok\n", delivery.key));
    }
}

#[cfg(test)]
mod tests {
    use super::Record;

    #[test]
    fn a_taken_key_is_not_resent() {
        assert!(!Record::read("m/high ok\n", "m/high").due(100, 30));
    }

    #[test]
    fn an_unanswered_send_waits_out_the_retry() {
        let sent = Record::read("m/high sent 100", "m/high");
        assert!(!sent.due(129, 30));
        assert!(sent.due(130, 30));
    }

    #[test]
    fn a_changed_key_is_sent() {
        assert!(Record::read("m/low ok", "m/high").due(0, 30));
    }
}
