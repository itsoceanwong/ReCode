use anyhow::Result;
use arboard::Clipboard;

use crate::models::{InjectOutcome, InjectionTarget, TargetKind};

pub fn list_targets() -> Result<Vec<InjectionTarget>> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, IsWindowVisible,
    };

    let mut targets: Vec<InjectionTarget> = Vec::new();

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let list = &mut *(lparam.0 as *mut Vec<InjectionTarget>);
        if IsWindowVisible(hwnd).as_bool() {
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut buf);
            if len > 0 {
                let title = String::from_utf16_lossy(&buf[..len as usize]);
                if !title.trim().is_empty() {
                    let kind = if title.to_ascii_lowercase().contains("terminal")
                        || title.to_ascii_lowercase().contains("powershell")
                        || title.to_ascii_lowercase().contains("cmd")
                    {
                        TargetKind::Terminal
                    } else {
                        TargetKind::DesktopApp
                    };
                    list.push(InjectionTarget {
                        kind,
                        reference: title,
                    });
                }
            }
        }
        BOOL(1)
    }

    unsafe {
        let ptr = &mut targets as *mut Vec<InjectionTarget> as isize;
        let _ = EnumWindows(Some(callback), LPARAM(ptr));
    }
    Ok(targets)
}

fn find_hwnd_by_title(title: &str) -> Option<windows::Win32::Foundation::HWND> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindow};

    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(wide.as_ptr())).ok()? };
    if unsafe { !IsWindow(hwnd).as_bool() } {
        return None;
    }
    Some(hwnd)
}

pub fn send(target: &InjectionTarget, text: &str) -> Result<InjectOutcome> {
    if let Some(hwnd) = find_hwnd_by_title(&target.reference) {
        return send_to_hwnd(hwnd, text);
    }

    let targets = list_targets().unwrap_or_default();
    let Some(matched) = targets
        .into_iter()
        .find(|t| t.reference.contains(&target.reference))
    else {
        return Ok(InjectOutcome::WindowNotFound);
    };
    let Some(hwnd) = find_hwnd_by_title(&matched.reference) else {
        return Ok(InjectOutcome::WindowNotFound);
    };
    send_to_hwnd(hwnd, text)
}

fn send_to_hwnd(hwnd: windows::Win32::Foundation::HWND, text: &str) -> Result<InjectOutcome> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
        std::thread::sleep(std::time::Duration::from_millis(150));
        let _ = GetForegroundWindow();
    }

    {
        let mut clip = Clipboard::new()?;
        clip.set_text(text.to_string())?;
    }
    std::thread::sleep(std::time::Duration::from_millis(50));

    unsafe {
        key_chord(0x11, 0x56); // Ctrl+V
        std::thread::sleep(std::time::Duration::from_millis(80));
        key_tap(0x0D); // Enter
    }
    Ok(InjectOutcome::Sent)
}

unsafe fn key_tap(vk: u16) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    };
    let mut inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: Default::default(),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let _ = SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32);
}

unsafe fn key_chord(mod_vk: u16, key_vk: u16) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    };
    let down = |vk: u16| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: Default::default(),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let up = |vk: u16| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let mut inputs = [down(mod_vk), down(key_vk), up(key_vk), up(mod_vk)];
    let _ = SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32);
}

pub fn open_accessibility_settings() -> Result<()> {
    Ok(())
}
