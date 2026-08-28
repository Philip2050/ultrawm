use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::*,
        UI::WindowsAndMessaging::*,
    },
};

struct BarState {
    workspaces: Vec<String>,
    active_workspace: usize,
    title: String,
    bg_color: u32,
    fg_color: u32,
    clock: String,
}

pub struct AppBar {
    pub hwnd: HWND,
    pub height: i32,
    pub width: i32,
}

impl AppBar {
    pub fn create(width: i32, height: i32, bg_color: u32, fg_color: u32) -> anyhow::Result<Self> {
        unsafe {
            let hinstance = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(bar_wnd_proc),
                hInstance: hinstance,
                lpszClassName: w!("UltraWMAppBar"),
                hbrBackground: HBRUSH(GetStockObject(HOLLOW_BRUSH).0),
                ..Default::default()
            };
            RegisterClassW(&class);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                w!("UltraWMAppBar"),
                w!("UltraWM Bar"),
                WS_POPUP | WS_VISIBLE,
                0, 0, width, height,
                None, None, hinstance, None,
            )?;

            let state = BarState {
                workspaces: vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string()],
                active_workspace: 0,
                title: String::new(),
                bg_color,
                fg_color,
                clock: String::new(),
            };
            let boxed = Box::new(state);
            let ptr = Box::into_raw(boxed);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);

            Ok(Self {
                hwnd,
                height,
                width,
            })
        }
    }

    pub fn update(&self) {
        unsafe {
            let _ = InvalidateRect(self.hwnd, None, TRUE);
        }
    }

    pub fn set_workspaces(&self, workspaces: Vec<String>, active: usize) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).workspaces = workspaces;
                (*ptr).active_workspace = active;
                self.update();
            }
        }
    }

    pub fn set_title(&self, title: &str) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).title = title.to_string();
                self.update();
            }
        }
    }

    pub fn set_clock(&self, clock: &str) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).clock = clock.to_string();
                self.update();
            }
        }
    }
}

impl Drop for AppBar {
    fn drop(&mut self) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn bar_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut BarState;
            if ptr.is_null() {
                return LRESULT(0);
            }
            let state = &*ptr;

            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let brush = CreateSolidBrush(COLORREF(state.bg_color));
            let rect = RECT {
                left: 0,
                top: 0,
                right: 9999,
                bottom: 9999,
            };
            FillRect(hdc, &rect, brush);
            let _ = DeleteObject(brush);

            SetBkMode(hdc, TRANSPARENT);
            let _ = SetTextColor(hdc, COLORREF(state.fg_color));

            // Draw workspace indicators
            let mut x = 10i32;
            for (i, ws) in state.workspaces.iter().enumerate() {
                let ws_text = format!(" {} ", ws);
                let ws_w: Vec<u16> = ws_text.encode_utf16().chain(Some(0)).collect();
                let mut ws_rect = RECT {
                    left: x,
                    top: 2,
                    right: x + 40,
                    bottom: 28,
                };

                if i == state.active_workspace {
                    let active_brush = CreateSolidBrush(COLORREF(state.fg_color));
                    let _ = FillRect(hdc, &RECT { left: x, top: 2, right: x + 36, bottom: 26 }, active_brush);
                    let _ = DeleteObject(active_brush);
                    let _ = SetTextColor(hdc, COLORREF(state.bg_color));
                }

                let _ = DrawTextW(
                    hdc,
                    &mut ws_w.clone(),
                    &mut ws_rect,
                    DT_VCENTER | DT_SINGLELINE | DT_CENTER,
                );

                if i == state.active_workspace {
                    let _ = SetTextColor(hdc, COLORREF(state.fg_color));
                }

                x += 40;
            }

            // Draw title
            let title_text = format!(" {} ", state.title);
            let title_w: Vec<u16> = title_text.encode_utf16().chain(Some(0)).collect();
            let mut title_rect = RECT {
                left: x + 10,
                top: 0,
                right: 9999,
                bottom: 9999,
            };
            let _ = DrawTextW(
                hdc,
                &mut title_w.clone(),
                &mut title_rect,
                DT_VCENTER | DT_SINGLELINE | DT_LEFT,
            );

            // Draw clock (right-aligned)
            if !state.clock.is_empty() {
                let clock_w: Vec<u16> = state.clock.encode_utf16().chain(Some(0)).collect();
                let mut clock_rect = RECT {
                    left: 9999,
                    top: 0,
                    right: 9999,
                    bottom: 9999,
                };
                let _ = DrawTextW(
                    hdc,
                    &mut clock_w.clone(),
                    &mut clock_rect,
                    DT_VCENTER | DT_SINGLELINE | DT_RIGHT,
                );
            }

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
