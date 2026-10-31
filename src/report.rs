//! Limen reports the model and effort to the herdr pane; the hook events it
//! hears carry no effort, so a change seen here is sent through its hook. One
//! record per session holds the last key sent and whether limen took it; an
//! unanswered send is retried after `RETRY` seconds.

use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
};

use serde::Serialize;
use serde_json::Value;

use crate::{
    host,
    status::{Effort, Status},
};

const RETRY: i64 = 30;
/// The hidden argument that runs the send in a detached child.
pub const RECEIPT: &str = "--receipt";

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

    fn due(&self, now: i64) -> bool {
        match self {
            Self::Taken => false,
            Self::Sent { at } => now - at >= RETRY,
            Self::Other => true,
        }
    }
}

#[derive(Serialize)]
struct Payload<'a> {
    hook_event_name: &'static str,
    model: &'a str,
    effort: &'a str,
    session_id: &'a Value,
    transcript_path: &'a Value,
    cwd: &'a Value,
}

/// Sends the session's model and effort to limen when they changed since limen last took them.
pub fn model(input: &Value, status: &Status, effort: Option<&Effort>, now: i64) {
    if host::var("HERDR_PANE_ID").is_none()
        || status.model.id.is_empty()
        || status.session.is_empty()
    {
        return;
    }
    if !host::on_path("limen") {
        return;
    }
    let level = effort.map_or("", Effort::explicit);
    let key = format!("{}/{level}", status.model.id);
    let record = record_path(&status.session);
    let seen = fs::read_to_string(&record).unwrap_or_default();
    if !Record::read(&seen, &key).due(now)
        || fs::write(&record, format!("{key} sent {now}\n")).is_err()
    {
        return;
    }
    let field = |name| input.get(name).unwrap_or(&Value::Null);
    let payload = Payload {
        hook_event_name: "StatusLine",
        model: &status.model.id,
        effort: level,
        session_id: field("session_id"),
        transcript_path: field("transcript_path"),
        cwd: field("cwd"),
    };
    let Ok(mut payload) = serde_json::to_vec(&payload) else {
        return;
    };
    payload.push(b'\n');
    let _ = spawn_receipt(&payload, record, &key);
}

fn record_path(session: &str) -> PathBuf {
    let dir = host::var("TMPDIR").unwrap_or_else(|| "/tmp".into());
    let dir = dir.strip_suffix('/').unwrap_or(&dir);
    PathBuf::from(format!("{dir}/claude-statusline-model.{session}"))
}

fn spawn_receipt(payload: &[u8], record: PathBuf, key: &str) -> std::io::Result<()> {
    let mut child = Command::new(std::env::current_exe()?)
        .arg(RECEIPT)
        .arg(record)
        .arg(key)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    child
        .stdin
        .take()
        .map_or(Ok(()), |mut stdin| stdin.write_all(payload))
}

/// The detached child: hands the payload to limen and records whether limen took it.
pub fn receipt(mut args: impl Iterator<Item = OsString>) {
    let (Some(record), Some(key)) = (args.next(), args.next()) else {
        return;
    };
    let mut payload = Vec::new();
    if std::io::stdin().read_to_end(&mut payload).is_err() {
        return;
    }
    let Ok(mut limen) = Command::new("limen")
        .args(["hook", "claude"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return;
    };
    if let Some(mut stdin) = limen.stdin.take() {
        let _ = stdin.write_all(&payload);
    }
    let Ok(output) = limen.wait_with_output() else {
        return;
    };
    if String::from_utf8_lossy(&output.stdout).contains("\"received\":true") {
        let _ = fs::write(record, format!("{} ok\n", key.to_string_lossy()));
    }
}

#[cfg(test)]
mod tests {
    use super::Record;

    #[test]
    fn a_taken_key_is_not_resent() {
        assert!(!Record::read("m/high ok\n", "m/high").due(100));
    }

    #[test]
    fn an_unanswered_send_waits_out_the_retry() {
        let sent = Record::read("m/high sent 100", "m/high");
        assert!(!sent.due(129));
        assert!(sent.due(130));
    }

    #[test]
    fn a_changed_key_is_sent() {
        assert!(Record::read("m/low ok", "m/high").due(0));
    }
}
