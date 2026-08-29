use std::ptr;
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
                self.render();
                self.update_alpha();
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
                let fade = elapsed - state.duration;
                let total_ms = 500u128;
                if fade.as_millis() >= total_ms {
                    state.alpha = 0;
                    self.update_alpha();
                    return false;
                }
                state.alpha = ((1.0 - fade.as_millis() as f32 / total_ms as f32) * 230.0) as u8;
                self.render();
                self.update_alpha();
                return true;
            }
            // Fade in during first 200ms
            if elapsed < Duration::from_millis(200) {
                state.alpha = ((elapsed.as_millis() as f32 / 200.0) * 230.0) as u8;
                self.render();
                self.update_alpha();
            }
            true
        }
    }

    pub fn render(&self) {
        unsafe {
            let hdc_screen = GetDC(None);

            // Create a 32-bit ARGB bitmap for per-pixel alpha rendering
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: self.width,
                    biHeight: -self.height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD::default(); 1],
            };

            let mut bits: *mut std::ffi::c_void = ptr::null_mut();
            let dib = match CreateDIBSection(
                hdc_screen,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits,
                None,
                0,
            ) {
                Ok(h) => h,
                Err(_) => {
                    let _ = ReleaseDC(None, hdc_screen);
                    return;
                }
            };

            // Draw content into the bitmap
            let mem_dc = CreateCompatibleDC(hdc_screen);
            let old_bmp = SelectObject(mem_dc, dib);

            // Get state
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut NotifState;
            if ptr.is_null() {
                SelectObject(mem_dc, old_bmp);
                let _ = DeleteObject(dib);
                let _ = DeleteDC(mem_dc);
                let _ = ReleaseDC(None, hdc_screen);
                return;
            }
            let state = &*ptr;

            // Clear with white (will be used as transparent via AlphaFormat)
            let _ = PatBlt(mem_dc, 0, 0, self.width, self.height, WHITENESS);

            // Draw rounded rectangle background
            let bg_brush = CreateSolidBrush(COLORREF(state.bg_color));
            let bg_pen = CreatePen(PS_SOLID, 0, COLORREF(state.bg_color));
            let old_brush = SelectObject(mem_dc, bg_brush);
            let old_pen = SelectObject(mem_dc, bg_pen);

            RoundRect(mem_dc, 0, 0, self.width, self.height, 12, 12);

            let _ = SelectObject(mem_dc, old_brush);
            let _ = SelectObject(mem_dc, old_pen);
            let _ = DeleteObject(bg_brush);
            let _ = DeleteObject(bg_pen);

            // Draw text
            let text_w: Vec<u16> = state.message.encode_utf16().chain(Some(0)).collect();
            let mut text_rect = RECT {
                left: 12,
                top: 4,
                right: self.width - 12,
                bottom: self.height - 4,
            };
            SetBkMode(mem_dc, TRANSPARENT);
            SetTextColor(mem_dc, COLORREF(state.fg_color));

            let font = CreateFontW(
                16, 0, 0, 0, FW_NORMAL.0 as i32,
                0u32, 0u32, 0u32,
                DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32, DEFAULT_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                w!("Segoe UI"),
            );
            let old_font = SelectObject(mem_dc, font);
            DrawTextW(
                mem_dc,
                &mut text_w.clone(),
                &mut text_rect,
                DT_VCENTER | DT_SINGLELINE | DT_LEFT,
            );
            let _ = SelectObject(mem_dc, old_font);
            let _ = DeleteObject(font);

            // Update the layered window
            let mut pt_src = POINT { x: 0, y: 0 };
            let mut sz = SIZE { cx: self.width, cy: self.height };
            let mut pt_dst = POINT { x: 0, y: 0 };

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: if self.is_alive() { self.get_alpha() } else { 0 },
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let _ = UpdateLayeredWindow(
                self.hwnd,
                hdc_screen,
                Some(&mut pt_dst),
                Some(&mut sz),
                mem_dc,
                Some(&mut pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            // Cleanup
            let _ = SelectObject(mem_dc, old_bmp);
            let _ = DeleteObject(dib);
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, hdc_screen);
        }
    }

    fn update_alpha(&self) {
        // Alpha is now applied per-pixel via the bitmap rendering
        // This method is kept for compatibility but doesn't need to do anything
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
        WM_SHOWWINDOW => {
            if _wparam.0 == 1 {
                if let Some(notifier) = get_notifier(hwnd) {
                    notifier.render();
                }
            }
            DefWindowProcW(hwnd, msg, _wparam, _lparam)
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

fn get_notifier(_hwnd: HWND) -> Option<&'static Notifier> {
    // We can't easily get a reference to Notifier here since it's owned elsewhere
    // Just return None - the render will be called from show/tick
    None
}
