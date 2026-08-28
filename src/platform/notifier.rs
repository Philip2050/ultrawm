use std::time::{Duration, Instant};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::*,
        UI::WindowsAndMessaging::*,
    },
};

struct NotifState {
    message: String,
    bg_color: u32,
    fg_color: u32,
    created: Instant,
    duration: Duration,
    alpha: u8,
}

pub struct Notifier {
    pub hwnd: HWND,
    width: i32,
    height: i32,
}

impl Notifier {
    pub fn create(width: i32, height: i32, bg: u32, fg: u32) -> anyhow::Result<Self> {
        unsafe {
            let hinstance = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(notif_wnd_proc),
                hInstance: hinstance,
                lpszClassName: w!("UltraWMNotifier"),
                hbrBackground: HBRUSH(GetStockObject(HOLLOW_BRUSH).0),
                ..Default::default()
            };
            RegisterClassW(&class);

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                w!("UltraWMNotifier"),
                w!("UltraWM Notifier"),
                WS_POPUP | WS_VISIBLE,
                0, 0, width, height,
                None, None, hinstance, None,
            )?;

            let state = NotifState {
                message: String::new(),
                bg_color: bg,
                fg_color: fg,
                created: Instant::now(),
                duration: Duration::from_secs(3),
                alpha: 0,
            };
            let boxed = Box::new(state);
            let ptr = Box::into_raw(boxed);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);

            Ok(Self { hwnd, width, height })
        }
    }

    pub fn show(&self, message: &str) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut NotifState;
            if !ptr.is_null() {
                (*ptr).message = message.to_string();
                (*ptr).created = Instant::now();
                (*ptr).duration = Duration::from_secs(3);
                (*ptr).alpha = 230;
                self.update_alpha();
                let _ = InvalidateRect(self.hwnd, None, TRUE);
            }
        }
    }

    pub fn tick(&self) -> bool {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut NotifState;
            if ptr.is_null() {
                return false;
            }
            let state = &mut *ptr;
            let elapsed = state.created.elapsed();
            if elapsed >= state.duration {
                // Fade out
                let fade = state.duration.subsec_millis() as u32;
                let total = 500;
                if fade >= total {
                    state.alpha = 0;
                    self.update_alpha();
                    return false;
                }
                state.alpha = ((1.0 - fade as f32 / total as f32) * 230.0) as u8;
                self.update_alpha();
                return true;
            }
            // Fade in during first 200ms
            if elapsed < Duration::from_millis(200) {
                state.alpha = ((elapsed.as_millis() as f32 / 200.0) * 230.0) as u8;
                self.update_alpha();
            }
            true
        }
    }

    fn update_alpha(&self) {
        unsafe {
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: if self.is_alive() { self.get_alpha() } else { 0 },
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let mut size = SIZE { cx: self.width, cy: self.height };
            let mut src_pt = POINT { x: 0, y: 0 };
            let mut dst_pt = POINT { x: 0, y: 0 };
            let _ = UpdateLayeredWindow(
                self.hwnd,
                GetDC(self.hwnd),
                Some(&mut dst_pt),
                Some(&mut size),
                GetDC(self.hwnd),
                Some(&mut src_pt),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }

    fn is_alive(&self) -> bool {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *const NotifState;
            if ptr.is_null() {
                return false;
            }
            (*ptr).alpha > 0
        }
    }

    fn get_alpha(&self) -> u8 {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *const NotifState;
            if ptr.is_null() {
                return 0;
            }
            (*ptr).alpha
        }
    }

    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                x, y, self.width, self.height,
                SWP_SHOWWINDOW | SWP_NOZORDER,
            );
        }
    }
}

impl Drop for Notifier {
    fn drop(&mut self) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut NotifState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn notif_wnd_proc(
    hwnd: HWND,
    msg: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NotifState;
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

            let text_w: Vec<u16> = state.message.encode_utf16().chain(Some(0)).collect();
            let mut text_rect = RECT {
                left: 12,
                top: 4,
                right: 9999,
                bottom: 9999,
            };
            let _ = DrawTextW(
                hdc,
                &mut text_w.clone(),
                &mut text_rect,
                DT_VCENTER | DT_SINGLELINE | DT_LEFT,
            );

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NotifState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, _wparam, _lparam),
    }
}
