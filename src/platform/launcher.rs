use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::*,
        UI::{
            Input::KeyboardAndMouse::*,
            Shell::*,
            WindowsAndMessaging::*,
        },
    },
};

fn hiword(val: u32) -> u16 {
    (val >> 16) as u16
}

pub static mut LAUNCHER_PTR: *mut AppLauncher = std::ptr::null_mut();

pub struct AppLauncher {
    pub hwnd: HWND,
    pub edit_hwnd: HWND,
    pub list_hwnd: HWND,
    pub apps: Vec<AppEntry>,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub path: String,
}

impl AppLauncher {
    pub fn create() -> anyhow::Result<()> {
        unsafe {
            let hinstance = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(launcher_wnd_proc),
                hInstance: hinstance,
                lpszClassName: w!("UltraWMLauncher"),
                hbrBackground: HBRUSH(GetStockObject(HOLLOW_BRUSH).0),
                ..Default::default()
            };
            RegisterClassW(&class);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE((WS_EX_TOPMOST.0 | WS_EX_LAYERED.0)),
                w!("UltraWMLauncher"),
                w!("UltraWM Launcher"),
                WINDOW_STYLE(WS_POPUP.0 | WS_VISIBLE.0),
                0, 0, 600, 400,
                None, None, hinstance, None,
            )?;

            let edit_hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("EDIT"),
                w!(""),
                WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32),
                10, 10, 560, 30,
                hwnd, None, hinstance, None,
            )?;

            let list_hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("LISTBOX"),
                w!(""),
                WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_VSCROLL.0 | WS_BORDER.0 | LBS_NOTIFY as u32),
                10, 50, 560, 330,
                hwnd, None, hinstance, None,
            )?;

            let apps = Self::scan_apps();

            for app in &apps {
                let text: Vec<u16> = app.name.encode_utf16().chain(Some(0)).collect();
                SendMessageW(list_hwnd, LB_ADDSTRING, WPARAM(0), LPARAM(text.as_ptr() as isize));
            }

            let launcher = Self {
                hwnd,
                edit_hwnd,
                list_hwnd,
                apps,
                visible: true,
            };

            let boxed = Box::new(launcher);
            let leaked = Box::leak(boxed);
            LAUNCHER_PTR = leaked as *mut AppLauncher;

            SetFocus(edit_hwnd);
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 240, LWA_ALPHA);
            Ok(())
        }
    }

    pub fn scan_apps() -> Vec<AppEntry> {
        let mut apps = Vec::new();

        let common = vec![
            ("Notepad", "notepad.exe"),
            ("Calculator", "calc.exe"),
            ("Command Prompt", "cmd.exe"),
            ("PowerShell", "powershell.exe"),
            ("Task Manager", "taskmgr.exe"),
            ("File Explorer", "explorer.exe"),
            ("Settings", "ms-settings:"),
            ("Microsoft Edge", "msedge.exe"),
        ];

        for (name, exe) in common {
            apps.push(AppEntry {
                name: name.to_string(),
                path: exe.to_string(),
            });
        }

        let start_menu_dirs = vec![
            dirs::data_dir().map(|d| d.join("Microsoft/Windows/Start Menu/Programs")),
            dirs::data_local_dir().map(|d| d.join("Microsoft/Windows/Start Menu/Programs")),
        ];

        for dir_opt in start_menu_dirs {
            if let Some(dir) = dir_opt {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(ext) = path.extension() {
                            if ext == "lnk" {
                                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                                    let path_str = path.to_string_lossy().to_string();
                                    if !apps.iter().any(|a| a.name == name) {
                                        apps.push(AppEntry {
                                            name: name.to_string(),
                                            path: path_str,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        apps.sort_by(|a, b| a.name.cmp(&b.name));
        apps
    }

    pub fn filter(&mut self, query: &str) {
        unsafe {
            let _ = SendMessageW(self.list_hwnd, LB_RESETCONTENT, WPARAM(0), LPARAM(0));
            let q = query.to_lowercase();
            for app in &self.apps {
                if app.name.to_lowercase().contains(&q) {
                    let text: Vec<u16> = app.name.encode_utf16().chain(Some(0)).collect();
                    SendMessageW(self.list_hwnd, LB_ADDSTRING, WPARAM(0), LPARAM(text.as_ptr() as isize));
                }
            }
        }
    }

    pub fn launch_selected(&self) {
        unsafe {
            let sel = SendMessageW(self.list_hwnd, LB_GETCURSEL, WPARAM(0), LPARAM(0));
            if sel.0 < 0 {
                return;
            }

            // Look up the app by its display text, not by index into the full list
            let mut buf = [0u16; 256];
            let _ = SendMessageW(self.list_hwnd, LB_GETTEXT, WPARAM(sel.0 as usize), LPARAM(buf.as_mut_ptr() as isize));
            let selected_name = String::from_utf16_lossy(&buf);
            let name = selected_name.trim_end_matches('\0');

            if let Some(app) = self.apps.iter().find(|a| a.name == name) {
                let _ = Self::launch(&app.path);
            }
        }
    }

    pub fn launch(path: &str) -> anyhow::Result<()> {
        use std::process::Command;
        if path.ends_with(".lnk") {
            unsafe {
                let path_w: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
                let _ = ShellExecuteW(
                    None,
                    w!("open"),
                    PCWSTR(path_w.as_ptr()),
                    None,
                    None,
                    SW_SHOWNORMAL,
                );
            }
        } else {
            Command::new(path).spawn()?;
        }
        Ok(())
    }

    pub fn dismiss(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

impl Drop for AppLauncher {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn launcher_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let notify = hiword(wparam.0 as u32);
            let ctrl_hwnd = HWND(lparam.0 as *mut _);
            if notify == LBN_DBLCLK as u16 {
                if let Some(l) = LAUNCHER_PTR.as_ref() {
                    l.launch_selected();
                }
            } else if notify == EN_CHANGE as u16 {
                if let Some(l) = LAUNCHER_PTR.as_mut() {
                    if ctrl_hwnd == l.edit_hwnd {
                        let mut buf = [0u16; 256];
                        let len = GetWindowTextW(l.edit_hwnd, &mut buf);
                        let query = String::from_utf16_lossy(&buf[..len as usize]);
                        l.filter(&query);
                    }
                }
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let launcher = LAUNCHER_PTR.as_ref();
            if let Some(l) = launcher {
                match wparam.0 as u32 {
                    val if val == VK_ESCAPE.0 as u32 => {
                        l.dismiss();
                    }
                    val if val == VK_RETURN.0 as u32 => {
                        l.launch_selected();
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_KILLFOCUS => {
            if let Some(l) = LAUNCHER_PTR.as_ref() {
                l.dismiss();
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
