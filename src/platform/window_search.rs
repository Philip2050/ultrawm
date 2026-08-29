use std::ptr::null_mut;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::*,
        UI::{
            Input::KeyboardAndMouse::*,
            WindowsAndMessaging::*,
        },
    },
};

pub static mut SEARCH_PTR: *mut WindowSearch = std::ptr::null_mut();

pub struct WindowSearch {
    pub hwnd: HWND,
    pub edit_hwnd: HWND,
    pub list_hwnd: HWND,
    pub windows: Vec<WindowEntry>,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct WindowEntry {
    pub title: String,
    pub exe: String,
    pub hwnd: HWND,
}

impl WindowSearch {
    pub fn create() -> anyhow::Result<()> {
        unsafe {
            let hinstance = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(search_wnd_proc),
                hInstance: hinstance,
                lpszClassName: w!("UltraWMWindowSearch"),
                hbrBackground: HBRUSH(GetStockObject(HOLLOW_BRUSH).0),
                ..Default::default()
            };
            RegisterClassW(&class);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE((WS_EX_TOPMOST.0 | WS_EX_LAYERED.0)),
                w!("UltraWMWindowSearch"),
                w!("UltraWM Window Search"),
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

            let search = Self {
                hwnd,
                edit_hwnd,
                list_hwnd,
                windows: Vec::new(),
                visible: true,
            };

            let boxed = Box::new(search);
            let leaked = Box::leak(boxed);
            SEARCH_PTR = leaked as *mut WindowSearch;

            SetFocus(edit_hwnd);
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 240, LWA_ALPHA);
            Ok(())
        }
    }

    pub fn populate(&mut self, platform: &crate::platform::Platform) {
        self.windows.clear();
        for (hwnd_wrapper, info) in &platform.windows {
            if info.visible && !info.minimized {
                let title = info.title.clone();
                let exe = info.exe.clone();
                self.windows.push(WindowEntry {
                    title,
                    exe,
                    hwnd: hwnd_wrapper.0,
                });
            }
        }
        self.windows.sort_by(|a, b| a.title.cmp(&b.title));
        self.refresh_list();
    }

    pub fn refresh_list(&self) {
        unsafe {
            let _ = SendMessageW(self.list_hwnd, LB_RESETCONTENT, WPARAM(0), LPARAM(0));
            for win in &self.windows {
                let text = format!("{} ({})", win.title, win.exe);
                let text_w: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
                let _ = SendMessageW(
                    self.list_hwnd,
                    LB_ADDSTRING,
                    WPARAM(0),
                    LPARAM(text_w.as_ptr() as isize),
                );
            }
        }
    }

    pub fn filter(&mut self, query: &str) {
        unsafe {
            let _ = SendMessageW(self.list_hwnd, LB_RESETCONTENT, WPARAM(0), LPARAM(0));
            let q = query.to_lowercase();
            for win in &self.windows {
                if win.title.to_lowercase().contains(&q)
                    || win.exe.to_lowercase().contains(&q)
                {
                    let text = format!("{} ({})", win.title, win.exe);
                    let text_w: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
                    let _ = SendMessageW(
                        self.list_hwnd,
                        LB_ADDSTRING,
                        WPARAM(0),
                        LPARAM(text_w.as_ptr() as isize),
                    );
                }
            }
        }
    }

    pub fn focus_selected(&self) {
        unsafe {
            let sel = SendMessageW(self.list_hwnd, LB_GETCURSEL, WPARAM(0), LPARAM(0));
            if sel.0 >= 0 {
                let mut buf = [0u16; 256];
                let _ = SendMessageW(
                    self.list_hwnd,
                    LB_GETTEXT,
                    WPARAM(sel.0 as usize),
                    LPARAM(buf.as_mut_ptr() as isize),
                );
                let selected = String::from_utf16_lossy(&buf);
                let selected = selected.trim_end_matches('\0');

                // Find the window by title+exe combination
                if let Some(win) = self.windows.iter().find(|w| {
                    let display = format!("{} ({})", w.title, w.exe);
                    display == selected
                }) {
                    let _ = SetForegroundWindow(win.hwnd);
                }
            }
        }
    }

    pub fn dismiss(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

impl Drop for WindowSearch {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn search_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let notify = ((wparam.0 >> 16) & 0xFFFF) as u16;
            let ctrl_hwnd = HWND(lparam.0 as *mut _);
            if notify == LBN_DBLCLK as u16 {
                if let Some(s) = SEARCH_PTR.as_ref() {
                    s.focus_selected();
                    s.dismiss();
                }
            } else if notify == EN_CHANGE as u16 {
                if let Some(s) = SEARCH_PTR.as_mut() {
                    if ctrl_hwnd == s.edit_hwnd {
                        let mut buf = [0u16; 256];
                        let len = GetWindowTextW(s.edit_hwnd, &mut buf);
                        let query = String::from_utf16_lossy(&buf[..len as usize]);
                        s.filter(&query);
                    }
                }
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let search = SEARCH_PTR.as_ref();
            if let Some(s) = search {
                match wparam.0 as u32 {
                    val if val == VK_ESCAPE.0 as u32 => {
                        s.dismiss();
                    }
                    val if val == VK_RETURN.0 as u32 => {
                        s.focus_selected();
                        s.dismiss();
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_KILLFOCUS => {
            if let Some(s) = SEARCH_PTR.as_ref() {
                s.dismiss();
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
