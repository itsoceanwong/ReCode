use anyhow::Result;
use arboard::Clipboard;
use std::process::Command;

use crate::injector::is_codex_or_claude_target;
use crate::models::{InjectOutcome, InjectionTarget, TargetKind};

pub fn list_targets() -> Result<Vec<InjectionTarget>> {
    let output = Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to get name of every process whose background only is false",
        ])
        .output()?;
    if !output.status.success() {
        return Ok(vec![]);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut targets = Vec::new();
    for name in text.split(',') {
        let name = name.trim().trim_matches('"');
        if name.is_empty() {
            continue;
        }
        if !is_codex_or_claude_target(name, Some(name)) {
            continue;
        }
        let kind = if name.contains("Terminal")
            || name.contains("iTerm")
            || name.contains("Alacritty")
            || name.contains("Warp")
        {
            TargetKind::Terminal
        } else {
            TargetKind::DesktopApp
        };
        targets.push(InjectionTarget {
            kind,
            reference: name.to_string(),
        });
    }
    Ok(targets)
}

pub fn send(target: &InjectionTarget, text: &str) -> Result<InjectOutcome> {
    if !is_codex_or_claude_target(&target.reference, Some(&target.reference)) {
        return Ok(InjectOutcome::WindowNotFound);
    }

    {
        let mut clip = Clipboard::new()?;
        clip.set_text(text.to_string())?;
    }

    let app = &target.reference;
    let activate = format!("tell application \"{app}\" to activate");
    let status = Command::new("osascript").args(["-e", &activate]).status()?;
    if !status.success() {
        return Ok(InjectOutcome::WindowNotFound);
    }

    std::thread::sleep(std::time::Duration::from_millis(200));

    let paste = r#"tell application "System Events" to keystroke "v" using command down"#;
    let enter = r#"tell application "System Events" to key code 36"#;
    let paste_ok = Command::new("osascript").args(["-e", paste]).status()?;
    if !paste_ok.success() {
        return Ok(InjectOutcome::NoPermission);
    }
    let enter_ok = Command::new("osascript").args(["-e", enter]).status()?;
    if !enter_ok.success() {
        return Ok(InjectOutcome::NoPermission);
    }
    Ok(InjectOutcome::Sent)
}

pub fn open_accessibility_settings() -> Result<()> {
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .status()?;
    Ok(())
}
