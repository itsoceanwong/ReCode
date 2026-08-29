use std::fs;
use std::path::PathBuf;

/// Current user's home directory (never hardcoded).
pub fn home_dir() -> PathBuf {
    directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .or_else(|| directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()))
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// OS config dir (Roaming AppData / Application Support / ~/.config).
fn config_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|b| b.config_dir().to_path_buf())
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .unwrap_or_else(|| home_dir().join(".config"))
}

/// OS local data dir (Local AppData / equivalent).
fn data_local_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|b| b.data_local_dir().to_path_buf())
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .unwrap_or_else(|| home_dir().join(".local").join("share"))
}

pub fn claude_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    home_dir().join(".claude")
}

pub fn claude_settings() -> PathBuf {
    claude_dir().join("settings.json")
}

pub fn codex_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CODEX_HOME") {
        return PathBuf::from(dir);
    }
    home_dir().join(".codex")
}

pub fn recode_dir() -> PathBuf {
    let dir = home_dir().join(".recode");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn recode_statusfile() -> PathBuf {
    recode_dir().join("claude-status.jsonl")
}

pub fn ccswitch_db() -> PathBuf {
    home_dir().join(".cc-switch").join("cc-switch.db")
}

fn push_cursor_state(out: &mut Vec<PathBuf>, root: PathBuf) {
    out.push(
        root.join("User")
            .join("globalStorage")
            .join("state.vscdb"),
    );
}

/// Candidate paths for Cursor's global `state.vscdb` (usage lives in cursorDiskKV).
/// All roots come from env / `directories` — never a machine-specific absolute path.
pub fn cursor_state_db_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();

    // Explicit override for portable / custom installs.
    if let Ok(dir) = std::env::var("CURSOR_USER_DATA_DIR") {
        push_cursor_state(&mut out, PathBuf::from(dir));
    }
    if let Ok(dir) = std::env::var("CURSOR_DATA_DIR") {
        push_cursor_state(&mut out, PathBuf::from(dir));
    }

    let config = config_dir();
    let local = data_local_dir();

    // Standard Cursor + Insiders under the OS config root.
    for name in ["Cursor", "Cursor - Insiders", "cursor"] {
        push_cursor_state(&mut out, config.join(name));
        push_cursor_state(&mut out, local.join(name));
    }

    // macOS historically uses Application Support (same as directories config_dir on macOS).
    // Extra Linux XDG-style homes already covered via config_dir / home fallbacks.
    #[cfg(target_os = "macos")]
    {
        let support = home_dir().join("Library").join("Application Support");
        for name in ["Cursor", "Cursor - Insiders"] {
            push_cursor_state(&mut out, support.join(name));
        }
    }

    // Dedup while preserving order.
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert(p.clone()));
    out
}

pub fn cursor_state_db() -> Option<PathBuf> {
    cursor_state_db_candidates().into_iter().find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recode_dir_exists_after_call() {
        let dir = recode_dir();
        assert!(dir.exists());
        assert!(dir.ends_with(".recode"));
    }

    #[test]
    fn path_helpers_are_not_machine_locked() {
        // Must not embed a developer machine absolute prefix.
        let samples = [
            home_dir().display().to_string(),
            claude_dir().display().to_string(),
            codex_dir().display().to_string(),
            recode_dir().display().to_string(),
        ];
        for s in samples {
            assert!(
                !s.contains(r"\Coding\ReCode"),
                "unexpected repo path leak: {s}"
            );
        }
        for p in cursor_state_db_candidates() {
            let s = p.display().to_string();
            assert!(
                !s.contains(r"\Coding\ReCode"),
                "cursor candidate leaked repo path: {s}"
            );
            // Relative product segments only after the resolved user root.
            assert!(
                s.contains("state.vscdb"),
                "cursor candidate missing state.vscdb: {s}"
            );
        }
    }

    #[test]
    fn cursor_override_env_is_respected() {
        let prev = std::env::var_os("CURSOR_USER_DATA_DIR");
        let fake = std::env::temp_dir().join("recode-cursor-override-test");
        let expected = fake
            .join("User")
            .join("globalStorage")
            .join("state.vscdb");
        std::env::set_var("CURSOR_USER_DATA_DIR", &fake);
        let first = cursor_state_db_candidates()
            .into_iter()
            .next()
            .expect("override should be first candidate");
        assert_eq!(first, expected);
        match prev {
            Some(v) => std::env::set_var("CURSOR_USER_DATA_DIR", v),
            None => std::env::remove_var("CURSOR_USER_DATA_DIR"),
        }
    }
}
