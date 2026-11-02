//! Events: when the text of a section marked `emit` changes, linea renders the
//! events body and hands it to the events command. One record per session holds
//! the last state sent and whether the command acknowledged it; an
//! unacknowledged send is retried once `retry` has passed.

use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{
    child,
    config::{self, Acknowledgement},
    host,
    line::Part,
    session::SessionId,
    template::Scope,
};

pub const SUBCOMMAND: &str = "deliver";

/// What a detached child needs to deliver one event.
#[derive(Debug, Serialize, Deserialize)]
pub struct Delivery {
    pub record: PathBuf,
    /// The emitting sections' texts, joined: the state this event reports.
    pub state: String,
    pub command: Vec<String>,
    pub acknowledgement: Acknowledgement,
    pub body: String,
}

#[derive(Debug, PartialEq, Eq)]
enum Record {
    Acknowledged,
    Sent { at: i64 },
    Other,
}

impl Record {
    fn read(seen: &str, state: &str) -> Self {
        let seen = seen.trim_end_matches('\n');
        if seen == format!("{state} ok") {
            Self::Acknowledged
        } else if let Some(rest) = seen.strip_prefix(&format!("{state} sent ")) {
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
            Self::Acknowledged => false,
            Self::Sent { at } => now - at >= retry,
            Self::Other => true,
        }
    }
}

/// The state the emitting sections report: their texts in drawing order, an absent one as empty.
pub fn state(config: &config::Config, parts: &[Part]) -> Option<String> {
    let texts: Vec<&str> = config
        .sections
        .iter()
        .filter(|section| section.emit)
        .map(|section| {
            parts
                .iter()
                .find(|part| part.name == section.name)
                .map_or("", |part| part.text.as_str())
        })
        .collect();
    (!texts.is_empty()).then(|| texts.join("/"))
}

/// Sends an event when the state changed since the command last acknowledged it.
pub fn send(
    events: &config::Events,
    state: Option<&str>,
    scope: &Scope,
    session: &SessionId,
    now: Timestamp,
) {
    let (Some(state), Some(program)) = (state, events.command.first()) else {
        return;
    };
    if !events.enabled
        || events
            .requires
            .as_deref()
            .is_some_and(|name| host::var(name).is_none())
        || session.is_empty()
        || !host::on_path(program)
    {
        return;
    }
    let record = PathBuf::from(format!(
        "{}/linea-events.{}",
        host::temp_dir(),
        session.file_name()
    ));
    let seen = fs::read_to_string(&record).unwrap_or_default();
    let now = now.as_second();
    if !Record::read(&seen, state).due(now, events.retry.as_secs()) {
        return;
    }
    let Ok(body) = scope.render(&events.body, None) else {
        return;
    };
    if fs::write(&record, format!("{state} sent {now}\n")).is_err() {
        return;
    }
    let delivery = Delivery {
        record,
        state: state.to_owned(),
        command: events.command.clone(),
        acknowledgement: events.acknowledgement.clone(),
        body,
    };
    let _ = child::detach(SUBCOMMAND, &delivery);
}

/// The detached child: hands the body to the command and records whether it acknowledged it.
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
    if let Some(mut stdin) = command.stdin.take() {
        let _ = stdin.write_all(format!("{}\n", delivery.body).as_bytes());
    }
    let Ok(output) = command.wait_with_output() else {
        return;
    };
    let acknowledged = match &delivery.acknowledgement {
        Acknowledgement::Marker(marker) => {
            String::from_utf8_lossy(&output.stdout).contains(marker.as_str())
        }
        Acknowledgement::ExitStatus => output.status.success(),
    };
    if acknowledged {
        let _ = fs::write(&delivery.record, format!("{} ok\n", delivery.state));
    }
}

#[cfg(test)]
mod tests {
    use super::Record;

    #[test]
    fn an_acknowledged_state_is_not_resent() {
        assert!(!Record::read("Opus/high ok\n", "Opus/high").due(100, 30));
    }

    #[test]
    fn an_unacknowledged_send_waits_out_the_retry() {
        let sent = Record::read("Opus/high sent 100", "Opus/high");
        assert!(!sent.due(129, 30));
        assert!(sent.due(130, 30));
    }

    #[test]
    fn a_changed_state_is_sent() {
        assert!(Record::read("Opus/low ok", "Opus/high").due(0, 30));
    }
}
