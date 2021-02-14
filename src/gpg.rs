// Ideally we can rewrite this with rgpg soon:
// https://github.com/rpgp/rpgp/issues/122
// This code is borrowed heavily from
// https://doc.rust-lang.org/rust-by-example/std_misc/process/pipe.html

use anyhow::Result;
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
};

// TODO: Parallel sign
// TODO: Give ETA
// TODO: Option for user to cancel and retry with fewer addresses

/// Pass the address you want signed along with a friendly message
/// Something like "This is a donation address for me, Satoshi Nakamoto:"
pub fn gpg_clearsign(address: &str, message: &str) -> Result<String> {
    // TODO: does this handle password input?
    // maybe some inspo here: https://github.com/BurntSushi/rust-cmail/blob/master/cmail.rs
    let process = Command::new("gpg")
        .arg("--clear-sign")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let str_to_sign = format!("{} {}", message, address);

    process.stdin.unwrap().write_all(str_to_sign.as_bytes())?;

    let mut s = String::new();
    process.stdout.unwrap().read_to_string(&mut s)?;

    Ok(s)
}
