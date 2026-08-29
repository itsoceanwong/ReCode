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
