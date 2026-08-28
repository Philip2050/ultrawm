use std::ptr;
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

pub static mut THEME_PICKER_PTR: *mut ThemePicker = ptr::null_mut();

pub struct ThemePicker {
    pub hwnd: HWND,
    pub list_hwnd: HWND,
    pub themes: Vec<String>,
}

impl ThemePicker {
    pub fn create(themes: Vec<String>, current: usize) -> anyhow::Result<()> {
        unsafe {
            let hinstance = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(theme_picker_wnd_proc),
                hInstance: hinstance,
                lpszClassName: w!("UltraWMThemePicker"),
                hbrBackground: HBRUSH(GetStockObject(HOLLOW_BRUSH).0),
                ..Default::default()
            };
            RegisterClassW(&class);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(WS_EX_TOPMOST.0),
                w!("UltraWMThemePicker"),
                w!("UltraWM Theme Picker"),
                WINDOW_STYLE(WS_POPUP.0 | WS_VISIBLE.0),
                0, 0, 400, 300,
                None, None, hinstance, None,
            )?;

            let list_hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("LISTBOX"),
                w!(""),
                WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_VSCROLL.0 | WS_BORDER.0 | LBS_NOTIFY as u32),
                10, 10, 360, 240,
                hwnd, None, hinstance, None,
            )?;

            for theme in &themes {
                let text: Vec<u16> = theme.encode_utf16().chain(Some(0)).collect();
                SendMessageW(list_hwnd, LB_ADDSTRING, WPARAM(0), LPARAM(text.as_ptr() as isize));
            }

            SendMessageW(list_hwnd, LB_SETCURSEL, WPARAM(current as usize), LPARAM(0));

            let picker = Self {
                hwnd,
                list_hwnd,
                themes,
            };

            let boxed = Box::new(picker);
            let leaked = Box::leak(boxed);
            THEME_PICKER_PTR = leaked as *mut ThemePicker;
            Ok(())
        }
    }

    pub fn dismiss(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

impl Drop for ThemePicker {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn theme_picker_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_KEYDOWN => {
            let picker = THEME_PICKER_PTR.as_ref();
            if let Some(p) = picker {
                let vk = wparam.0 as u32;
                match vk {
                    x if x == VK_ESCAPE.0 as u32 => {
                        p.dismiss();
                    }
                    x if x == VK_RETURN.0 as u32 => {
                        let sel = SendMessageW(p.list_hwnd, LB_GETCURSEL, WPARAM(0), LPARAM(0));
                        if sel.0 >= 0 {
                            let idx = sel.0 as usize;
                            if idx < p.themes.len() {
                                log::info!("Applying theme: {}", p.themes[idx]);
                                if let Some(platform) = crate::platform::keyboard::PLATFORM_PTR.as_mut() {
                                    let _ = platform.apply_theme_by_idx(idx);
                                }
                            }
                        }
                        p.dismiss();
                    }
                    _ => {}
                }
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
