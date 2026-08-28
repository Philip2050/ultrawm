use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::Diagnostics::ToolHelp::*,
        UI::WindowsAndMessaging::*,
    },
};

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: HWND,
    pub id: u64,
    pub title: String,
    pub class: String,
    pub exe: String,
    pub visible: bool,
    pub cloaked: bool,
    pub maximized: bool,
    pub minimized: bool,
    pub floating: bool,
    pub fullscreen: bool,
    pub always_on_top: bool,
    pub saved_x: i32,
    pub saved_y: i32,
    pub saved_w: i32,
    pub saved_h: i32,
    pub opacity: Option<f32>,
    pub sticky: bool,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
}

impl WindowInfo {
    pub fn from_hwnd(hwnd: HWND) -> anyhow::Result<Self> {
        unsafe {
            let mut title = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            let title_str = if len > 0 {
                String::from_utf16_lossy(&title[..len as usize])
            } else {
                String::new()
            };

            let mut class = [0u16; 256];
            let len = GetClassNameW(hwnd, &mut class);
            let class_str = if len > 0 {
                String::from_utf16_lossy(&class[..len as usize])
            } else {
                String::new()
            };

            let pid = {
                let mut pid: u32 = 0;
                let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
                pid
            };

            let exe = get_process_name(pid).unwrap_or_default();

            let visible = IsWindowVisible(hwnd).as_bool();
            let maximized = {
                let mut placement = WINDOWPLACEMENT {
                    length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
                    ..Default::default()
                };
                if GetWindowPlacement(hwnd, &mut placement).is_ok() {
                    placement.showCmd == SW_SHOWMAXIMIZED.0 as u32
                } else {
                    false
                }
            };

            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);

            Ok(Self {
                hwnd,
                id: 0,
                title: title_str,
                class: class_str,
                exe,
                visible,
                cloaked: false,
                maximized,
                minimized: false,
                floating: false,
                fullscreen: false,
                always_on_top: false,
                saved_x: rect.left,
                saved_y: rect.top,
                saved_w: rect.right - rect.left,
                saved_h: rect.bottom - rect.top,
                opacity: None,
                sticky: false,
                max_width: None,
                max_height: None,
                min_width: None,
                min_height: None,
            })
        }
    }

    pub fn should_tile(&self) -> bool {
        if !self.visible {
            return false;
        }
        let skip_classes = [
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
            "Progman",
            "WorkerW",
            "ApplicationFrameWindow",
            "Windows.UI.Core.CoreWindow",
        ];
        if skip_classes.iter().any(|&c| self.class == c) {
            return false;
        }
        if self.title.is_empty() && self.class.contains("Tool") {
            return false;
        }
        true
    }
}

fn get_process_name(pid: u32) -> Option<String> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == pid {
                    let name = String::from_utf16_lossy(&entry.szExeFile);
                    let end = name.find('\0').unwrap_or(name.len());
                    return Some(name[..end].to_string());
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        None
    }
}
