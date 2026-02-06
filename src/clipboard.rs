use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardBackend {
    Pbpaste,
    WlPaste,
    Xclip,
    Xsel,
}

#[derive(Debug)]
pub struct ClipboardError {
    pub message: String,
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ClipboardError {}

pub fn detect_backend() -> Result<ClipboardBackend, ClipboardError> {
    let wayland = env::var_os("WAYLAND_DISPLAY").is_some();
    let x11 = env::var_os("DISPLAY").is_some();

    let availability = Availability {
        pb: has_cmd("pbpaste") && has_cmd("pbcopy"),
        wl: has_cmd("wl-paste") && has_cmd("wl-copy"),
        xclip: has_cmd("xclip"),
        xsel: has_cmd("xsel"),
    };

    if let Some(b) = pick_backend(wayland, x11, availability) {
        return Ok(b);
    }

    Err(ClipboardError {
        message: "No supported clipboard utilities found. Install pbpaste/pbcopy (macOS), wl-paste/wl-copy (Wayland), or xclip/xsel (X11)."
            .to_string(),
    })
}

pub fn read_clipboard() -> Result<String, ClipboardError> {
    let backend = detect_backend()?;
    match backend {
        ClipboardBackend::Pbpaste => run_read(Command::new("pbpaste")),
        ClipboardBackend::WlPaste => run_read(Command::new("wl-paste")),
        ClipboardBackend::Xclip => read_xclip(),
        ClipboardBackend::Xsel => read_xsel(),
    }
}

pub fn write_clipboard(text: &str) -> Result<(), ClipboardError> {
    let backend = detect_backend()?;
    match backend {
        ClipboardBackend::Pbpaste => run_write(Command::new("pbcopy"), text),
        ClipboardBackend::WlPaste => run_write(Command::new("wl-copy"), text),
        ClipboardBackend::Xclip => {
            let mut cmd = Command::new("xclip");
            cmd.arg("-selection").arg("clipboard");
            run_write(cmd, text)
        }
        ClipboardBackend::Xsel => {
            let mut cmd = Command::new("xsel");
            cmd.arg("--clipboard").arg("--input");
            run_write(cmd, text)
        }
    }
}

fn run_read(mut cmd: Command) -> Result<String, ClipboardError> {
    let output = cmd.output().map_err(|e| ClipboardError {
        message: format!("Failed to read clipboard: {}", e),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        let suffix = if stderr.is_empty() {
            "".to_string()
        } else {
            format!(" Details: {}", stderr)
        };
        return Err(ClipboardError {
            message: format!("Clipboard read command failed.{}", suffix),
        });
    }

    String::from_utf8(output.stdout).map_err(|e| ClipboardError {
        message: format!("Clipboard content not valid UTF-8: {}", e),
    })
}

fn read_xclip() -> Result<String, ClipboardError> {
    let mut cmd = Command::new("xclip");
    cmd.arg("-selection").arg("clipboard").arg("-o");
    match run_read(cmd) {
        Ok(s) => Ok(s),
        Err(_) => {
            // Fallback to primary selection if clipboard is empty.
            let mut cmd2 = Command::new("xclip");
            cmd2.arg("-selection").arg("primary").arg("-o");
            run_read(cmd2)
        }
    }
}

fn read_xsel() -> Result<String, ClipboardError> {
    let mut cmd = Command::new("xsel");
    cmd.arg("--clipboard").arg("--output");
    match run_read(cmd) {
        Ok(s) => Ok(s),
        Err(_) => {
            // Fallback to primary selection if clipboard is empty.
            let mut cmd2 = Command::new("xsel");
            cmd2.arg("--primary").arg("--output");
            run_read(cmd2)
        }
    }
}

fn run_write(mut cmd: Command, text: &str) -> Result<(), ClipboardError> {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| ClipboardError {
            message: format!("Failed to write clipboard: {}", e),
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| ClipboardError {
                message: format!("Failed to write clipboard: {}", e),
            })?;
    }

    let status = child.wait().map_err(|e| ClipboardError {
        message: format!("Failed to write clipboard: {}", e),
    })?;

    if !status.success() {
        return Err(ClipboardError {
            message: "Clipboard write command failed.".to_string(),
        });
    }

    Ok(())
}

fn has_cmd(cmd: &str) -> bool {
    if let Some(path) = find_in_path(cmd) {
        return is_executable(&path);
    }
    false
}

#[derive(Clone, Copy)]
struct Availability {
    pb: bool,
    wl: bool,
    xclip: bool,
    xsel: bool,
}

fn pick_backend(wayland: bool, x11: bool, a: Availability) -> Option<ClipboardBackend> {
    if a.pb {
        return Some(ClipboardBackend::Pbpaste);
    }
    if wayland && a.wl {
        return Some(ClipboardBackend::WlPaste);
    }
    if x11 && a.xclip {
        return Some(ClipboardBackend::Xclip);
    }
    if x11 && a.xsel {
        return Some(ClipboardBackend::Xsel);
    }
    if a.wl {
        return Some(ClipboardBackend::WlPaste);
    }
    if a.xclip {
        return Some(ClipboardBackend::Xclip);
    }
    if a.xsel {
        return Some(ClipboardBackend::Xsel);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_wayland_tools_when_available() {
        let a = Availability {
            pb: false,
            wl: true,
            xclip: true,
            xsel: true,
        };
        let b = pick_backend(true, true, a);
        assert_eq!(b, Some(ClipboardBackend::WlPaste));
    }

    #[test]
    fn prefers_xclip_over_xsel_on_x11() {
        let a = Availability {
            pb: false,
            wl: false,
            xclip: true,
            xsel: true,
        };
        let b = pick_backend(false, true, a);
        assert_eq!(b, Some(ClipboardBackend::Xclip));
    }

    #[test]
    fn falls_back_to_xsel_when_only_xsel_available() {
        let a = Availability {
            pb: false,
            wl: false,
            xclip: false,
            xsel: true,
        };
        let b = pick_backend(false, false, a);
        assert_eq!(b, Some(ClipboardBackend::Xsel));
    }

    #[test]
    fn none_when_no_tools() {
        let a = Availability {
            pb: false,
            wl: false,
            xclip: false,
            xsel: false,
        };
        let b = pick_backend(false, false, a);
        assert_eq!(b, None);
    }
}

fn find_in_path(cmd: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(cmd);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mode = meta.permissions().mode();
        return mode & 0o111 != 0;
    }
    false
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}
