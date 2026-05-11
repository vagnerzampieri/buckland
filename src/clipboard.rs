//! Cross-desktop clipboard helper. Shells out to `wl-copy` (Wayland)
//! or `xclip -selection clipboard` (X11) — no clipboard crate dep.
//!
//! Detection is env-driven, matching the freedesktop convention:
//!
//! - `$WAYLAND_DISPLAY` non-empty → `wl-copy`
//! - else `$DISPLAY` non-empty     → `xclip -selection clipboard`
//! - else                          → `ClipboardError::NoServer`
//!
//! If detection picks a tool but the binary is missing on `PATH`, we
//! return `ClipboardError::ToolMissing(name)`. Callers use this to
//! render a footer error like "Copy failed: wl-copy not found".

use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    WlCopy,
    Xclip,
}

impl Tool {
    pub fn binary(self) -> &'static str {
        match self {
            Tool::WlCopy => "wl-copy",
            Tool::Xclip => "xclip",
        }
    }

    fn args(self) -> &'static [&'static str] {
        match self {
            Tool::WlCopy => &[],
            Tool::Xclip => &["-selection", "clipboard"],
        }
    }
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.binary())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ClipboardError {
    #[error("no display server detected (WAYLAND_DISPLAY and DISPLAY both unset)")]
    NoServer,
    #[error("{0} not found on PATH")]
    ToolMissing(&'static str),
    #[error("{tool} exited with code {code}")]
    ToolFailed { tool: &'static str, code: i32 },
    #[error("io error talking to clipboard tool: {0}")]
    Io(String),
}

impl From<std::io::Error> for ClipboardError {
    fn from(e: std::io::Error) -> Self {
        ClipboardError::Io(e.to_string())
    }
}

/// Pick the clipboard tool from the env. `read_env` is injected so
/// tests don't need to mutate the real process env.
pub fn detect_tool(read_env: &dyn Fn(&str) -> Option<String>) -> Result<Tool, ClipboardError> {
    let wayland = read_env("WAYLAND_DISPLAY").unwrap_or_default();
    if !wayland.is_empty() {
        return Ok(Tool::WlCopy);
    }
    let display = read_env("DISPLAY").unwrap_or_default();
    if !display.is_empty() {
        return Ok(Tool::Xclip);
    }
    Err(ClipboardError::NoServer)
}

/// Copy `text` to the system clipboard. Returns the tool that was used
/// on success so callers can render "Copied via wl-copy" in the UI.
pub fn copy(text: &str) -> Result<Tool, ClipboardError> {
    let env = |k: &str| std::env::var(k).ok();
    let tool = detect_tool(&env)?;
    spawn_and_pipe(tool, text)?;
    Ok(tool)
}

fn spawn_and_pipe(tool: Tool, text: &str) -> Result<(), ClipboardError> {
    let mut cmd = Command::new(tool.binary());
    cmd.args(tool.args())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ClipboardError::ToolMissing(tool.binary()));
        }
        Err(e) => return Err(e.into()),
    };
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(ClipboardError::ToolFailed {
            tool: tool.binary(),
            code: status.code().unwrap_or(-1),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_for<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k: &str| {
            pairs
                .iter()
                .find(|(name, _)| *name == k)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn detect_picks_wl_copy_when_wayland_display_set() {
        let env = env_for(&[("WAYLAND_DISPLAY", "wayland-0")]);
        assert_eq!(detect_tool(&env), Ok(Tool::WlCopy));
    }

    #[test]
    fn detect_falls_back_to_xclip_when_only_display_set() {
        let env = env_for(&[("DISPLAY", ":0")]);
        assert_eq!(detect_tool(&env), Ok(Tool::Xclip));
    }

    #[test]
    fn detect_prefers_wayland_when_both_are_set() {
        let env = env_for(&[("WAYLAND_DISPLAY", "wayland-0"), ("DISPLAY", ":0")]);
        assert_eq!(detect_tool(&env), Ok(Tool::WlCopy));
    }

    #[test]
    fn detect_fails_when_neither_env_var_is_set() {
        let env = env_for(&[]);
        assert_eq!(detect_tool(&env), Err(ClipboardError::NoServer));
    }

    #[test]
    fn detect_treats_empty_env_var_as_unset() {
        let env = env_for(&[("WAYLAND_DISPLAY", ""), ("DISPLAY", "")]);
        assert_eq!(detect_tool(&env), Err(ClipboardError::NoServer));
    }
}
