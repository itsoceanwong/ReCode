use crate::models::{InjectOutcome, InjectionTarget, TargetKind};

#[cfg(target_os = "macos")]
mod platform {
    pub use crate::injector_macos::*;
}

#[cfg(target_os = "windows")]
mod platform {
    pub use crate::injector_windows::*;
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    use crate::models::{InjectOutcome, InjectionTarget};
    use anyhow::Result;

    pub fn list_targets() -> Result<Vec<InjectionTarget>> {
        Ok(vec![])
    }

    pub fn send(_target: &InjectionTarget, _text: &str) -> Result<InjectOutcome> {
        Ok(InjectOutcome::Error {
            detail: "injection unsupported on this OS".into(),
        })
    }

    pub fn open_accessibility_settings() -> Result<()> {
        Ok(())
    }
}

/// True when title or process path/name looks like Codex or Claude Code.
pub fn is_codex_or_claude_target(title_or_name: &str, process_path_or_name: Option<&str>) -> bool {
    if looks_like_codex_or_claude(title_or_name) {
        return true;
    }
    let Some(proc) = process_path_or_name else {
        return false;
    };
    if looks_like_codex_or_claude(proc) {
        return true;
    }
    std::path::Path::new(proc)
        .file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(looks_like_codex_or_claude)
}

fn looks_like_codex_or_claude(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.contains("codex") || lower.contains("claude")
}

pub trait Injector {
    fn send(&self, target: &InjectionTarget, text: &str) -> anyhow::Result<InjectOutcome>;
    fn list_targets(&self) -> anyhow::Result<Vec<InjectionTarget>>;
}

pub struct OsInjector;

impl Injector for OsInjector {
    fn send(&self, target: &InjectionTarget, text: &str) -> anyhow::Result<InjectOutcome> {
        platform::send(target, text)
    }

    fn list_targets(&self) -> anyhow::Result<Vec<InjectionTarget>> {
        platform::list_targets()
    }
}

pub fn list_targets() -> anyhow::Result<Vec<InjectionTarget>> {
    platform::list_targets()
}

pub fn send(target: &InjectionTarget, text: &str) -> anyhow::Result<InjectOutcome> {
    platform::send(target, text)
}

pub fn open_accessibility_settings() -> anyhow::Result<()> {
    platform::open_accessibility_settings()
}

pub fn kind_label(kind: &TargetKind) -> &'static str {
    match kind {
        TargetKind::DesktopApp => "desktop_app",
        TargetKind::Terminal => "terminal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_title_keywords() {
        assert!(is_codex_or_claude_target("Codex — StudySystem", None));
        assert!(is_codex_or_claude_target("Claude Code", None));
        assert!(is_codex_or_claude_target("pwsh - claude", None));
        assert!(!is_codex_or_claude_target("Google Chrome", None));
        assert!(!is_codex_or_claude_target("Windows Terminal", None));
    }

    #[test]
    fn matches_process_name_or_path() {
        assert!(is_codex_or_claude_target(
            "Windows Terminal",
            Some(r"C:\Tools\claude.exe")
        ));
        assert!(is_codex_or_claude_target("app", Some("/usr/local/bin/codex")));
        assert!(!is_codex_or_claude_target(
            "Windows Terminal",
            Some(r"C:\Windows\System32\WindowsTerminal.exe")
        ));
    }
}
