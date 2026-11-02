//! Work linea hands to a detached copy of itself: the message travels as JSON
//! on the child's stdin, and the line never waits for the child.

use std::{
    io::{self, Read, Write},
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use serde::{Serialize, de::DeserializeOwned};

/// Starts `linea <subcommand>` in its own process group and hands it `message`.
pub fn detach(subcommand: &str, message: &impl Serialize) -> io::Result<()> {
    let message = serde_json::to_vec(message)?;
    let mut child = Command::new(std::env::current_exe()?)
        .arg(subcommand)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    child
        .stdin
        .take()
        .map_or(Ok(()), |mut stdin| stdin.write_all(&message))
}

/// The message a detached child was started with.
pub fn receive<T: DeserializeOwned>() -> Option<T> {
    let mut message = Vec::new();
    io::stdin().read_to_end(&mut message).ok()?;
    serde_json::from_slice(&message).ok()
}
