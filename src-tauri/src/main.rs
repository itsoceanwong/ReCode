// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let debug = std::env::args().skip(1).any(|a| a == "debug" || a == "--debug");
    if debug {
        #[cfg(windows)]
        win_debug_console::attach();
        // SAFETY: called once at process start, before other threads read env.
        unsafe { std::env::set_var("RECODE_DEBUG", "1") };
        eprintln!("ReCode: debug console enabled (arg: debug|--debug)");
    }
    recode_lib::run()
}

#[cfg(windows)]
mod win_debug_console {
    use std::ffi::CString;
    use windows::Win32::System::Console::{
        AllocConsole, AttachConsole, ATTACH_PARENT_PROCESS,
    };

    // MSVC UCRT: stdin=0, stdout=1, stderr=2
    #[link(name = "ucrt")]
    extern "C" {
        fn freopen(
            filename: *const i8,
            mode: *const i8,
            stream: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
        fn __acrt_iob_func(index: u32) -> *mut std::ffi::c_void;
    }

    /// Attach to the parent terminal when launched from cmd/PowerShell;
    /// otherwise allocate a new console window. Then reconnect CRT stdio
    /// so `println!` / `eprintln!` actually appear (release is a GUI subsystem binary).
    pub fn attach() {
        unsafe {
            if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
                let _ = AllocConsole();
            }
            redirect_stdio_to_console();
        }
    }

    unsafe fn redirect_stdio_to_console() {
        let conout = CString::new("CONOUT$").expect("CONOUT$");
        let conin = CString::new("CONIN$").expect("CONIN$");
        let mode_w = CString::new("w").expect("w");
        let mode_r = CString::new("r").expect("r");

        let _ = freopen(conin.as_ptr(), mode_r.as_ptr(), __acrt_iob_func(0));
        let _ = freopen(conout.as_ptr(), mode_w.as_ptr(), __acrt_iob_func(1));
        let _ = freopen(conout.as_ptr(), mode_w.as_ptr(), __acrt_iob_func(2));
    }
}
