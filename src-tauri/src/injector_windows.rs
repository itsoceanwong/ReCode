use anyhow::Result;
use arboard::Clipboard;

use crate::injector::is_codex_or_claude_target;
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
                let process = process_image_for_hwnd(hwnd);
                if is_codex_or_claude_target(title.trim(), process.as_deref()) {
                    let kind = classify_kind(&title, process.as_deref());
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

fn classify_kind(title: &str, process: Option<&str>) -> TargetKind {
    let blob = format!(
        "{} {}",
        title.to_ascii_lowercase(),
        process.unwrap_or("").to_ascii_lowercase()
    );
    if blob.contains("terminal")
        || blob.contains("powershell")
        || blob.contains("pwsh")
        || blob.contains("cmd")
        || blob.contains("windows terminal")
    {
        TargetKind::Terminal
    } else {
        TargetKind::DesktopApp
    }
}

fn process_image_for_hwnd(hwnd: windows::Win32::Foundation::HWND) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    if pid == 0 {
        return None;
    }
    let handle =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buf = [0u16; 1024];
    let mut size = buf.len() as u32;
    let ok = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
    };
    let _ = unsafe { CloseHandle(handle) };
    if ok.is_err() || size == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..size as usize]))
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

fn hwnd_is_allowed(hwnd: windows::Win32::Foundation::HWND, title_hint: &str) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;

    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    let title = if len > 0 {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        title_hint.to_string()
    };
    let process = process_image_for_hwnd(hwnd);
    is_codex_or_claude_target(title.trim(), process.as_deref())
}

pub fn send(target: &InjectionTarget, text: &str) -> Result<InjectOutcome> {
    if let Some(hwnd) = find_hwnd_by_title(&target.reference) {
        if !hwnd_is_allowed(hwnd, &target.reference) {
            return Ok(InjectOutcome::WindowNotFound);
        }
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
    if !hwnd_is_allowed(hwnd, &matched.reference) {
        return Ok(InjectOutcome::WindowNotFound);
    }
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
