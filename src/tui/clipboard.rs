//! Thin wrapper around `wl-copy` and `xclip`. Best-effort: missing
//! tools surface a structured error the caller can show in the footer
//! instead of panicking.

use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("no clipboard tool found (need wl-copy or xclip)")]
    NoTool,
    #[error("clipboard tool exited with code {0}")]
    Exit(i32),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn copy(text: &str) -> Result<&'static str, ClipboardError> {
    if let Some(name) = try_copy_with("wl-copy", &[], text)? {
        return Ok(name);
    }
    if let Some(name) = try_copy_with("xclip", &["-selection", "clipboard"], text)? {
        return Ok(name);
    }
    Err(ClipboardError::NoTool)
}

fn try_copy_with(
    bin: &'static str,
    extra_args: &[&str],
    text: &str,
) -> Result<Option<&'static str>, ClipboardError> {
    let mut cmd = Command::new(bin);
    cmd.args(extra_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(Some(bin))
    } else {
        Err(ClipboardError::Exit(status.code().unwrap_or(-1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_tool_when_neither_wl_copy_nor_xclip_present() {
        // We test by asking for a binary that definitely doesn't exist.
        let res = try_copy_with("definitely-not-a-real-binary-xyz", &[], "hi");
        assert!(matches!(res, Ok(None)));
    }
}
