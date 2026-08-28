use std::ptr::null_mut;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::*,
        UI::WindowsAndMessaging::*,
    },
};

pub struct BorderOverlay {
    pub hwnd: HWND,
    pub border_width: i32,
    pub border_radius: i32,
    width: i32,
    height: i32,
    mem_dc: HDC,
    bitmap: HBITMAP,
    old_bitmap: HGDIOBJ,
    bits: *mut u8,
    title_font: HFONT,
}

fn color_dim(c: u32, factor: f32) -> u32 {
    let r = ((c & 0xFF) as f32 * factor) as u32;
    let g = (((c >> 8) & 0xFF) as f32 * factor) as u32;
    let b = (((c >> 16) & 0xFF) as f32 * factor) as u32;
    (b << 16) | ((g & 0xFF) << 8) | (r & 0xFF)
}

impl BorderOverlay {
    pub fn create(width: i32, height: i32) -> anyhow::Result<Self> {
        unsafe {
            let hinstance = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(border_wnd_proc),
                hInstance: hinstance,
                lpszClassName: w!("UltraWMBorderOverlay"),
                hbrBackground: HBRUSH(GetStockObject(HOLLOW_BRUSH).0),
                ..Default::default()
            };

            RegisterClassW(&class);

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                w!("UltraWMBorderOverlay"),
                w!("UltraWM Overlay"),
                WS_POPUP,
                0, 0, width, height,
                None, None, hinstance, None,
            )?;

            let hdc_screen = GetDC(None);
            let mem_dc = CreateCompatibleDC(hdc_screen);

            let mut bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                bmiColors: [Default::default(); 1],
            };

            let mut bits: *mut std::ffi::c_void = null_mut();
            let bitmap = CreateDIBSection(mem_dc, &bi, DIB_RGB_COLORS, &mut bits, None, 0)?;

            let old_bitmap = SelectObject(mem_dc, bitmap);
            ReleaseDC(None, hdc_screen);

            ShowWindow(hwnd, SW_SHOW);

            let title_font = CreateFontW(
                -12, 0, 0, 0, FW_NORMAL.0 as i32,
                0u32, 0u32, 0u32,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                DEFAULT_QUALITY.0 as u32,
                0u32,
                w!("Segoe UI"),
            );

            Ok(Self {
                hwnd,
                border_width: 2,
                border_radius: 8,
                width,
                height,
                mem_dc,
                bitmap,
                old_bitmap,
                bits: bits as *mut u8,
                title_font,
            })
        }
    }

    pub fn update(&mut self, rects: &[(i32, i32, i32, i32, u32, bool, Option<String>)]) {
        unsafe {
            std::ptr::write_bytes(self.bits, 0, (self.width * self.height * 4) as usize);

            for &(x, y, w, h, color_rgb, focused, ref title) in rects {
                let bw = self.border_width;
                let half = bw / 2;

                if focused {
                    // Outer glow: wider, dimmed stroke
                    let glow_pen = CreatePen(PS_SOLID, bw + 4, COLORREF(color_dim(color_rgb, 0.25)));
                    let old_pen = SelectObject(self.mem_dc, glow_pen);
                    let rr = self.border_radius;
                    RoundRect(
                        self.mem_dc,
                        x + half - 1,
                        y + half - 1,
                        x + w - half + 1,
                        y + h - half + 1,
                        rr, rr,
                    );
                    SelectObject(self.mem_dc, old_pen);
                    let _ = DeleteObject(glow_pen);

                    // Mid glow
                    let mid_pen = CreatePen(PS_SOLID, bw + 2, COLORREF(color_dim(color_rgb, 0.5)));
                    let old_pen = SelectObject(self.mem_dc, mid_pen);
                    RoundRect(
                        self.mem_dc,
                        x + half,
                        y + half,
                        x + w - half,
                        y + h - half,
                        rr, rr,
                    );
                    SelectObject(self.mem_dc, old_pen);
                    let _ = DeleteObject(mid_pen);

                    // Inner border
                    let pen = CreatePen(PS_SOLID, bw, COLORREF(color_rgb));
                    let old_pen = SelectObject(self.mem_dc, pen);
                    RoundRect(
                        self.mem_dc,
                        x + half,
                        y + half,
                        x + w - half,
                        y + h - half,
                        rr, rr,
                    );
                    SelectObject(self.mem_dc, old_pen);
                    let _ = DeleteObject(pen);

                    // Window title
                    if let Some(ref t) = title {
                        let old_font = SelectObject(self.mem_dc, self.title_font);
                        SetBkMode(self.mem_dc, TRANSPARENT);
                        SetTextColor(self.mem_dc, COLORREF(color_rgb));
                        let text_y = y + half + 2;
                        let text_x = x + half + 6;
                        let mut wide: Vec<u16> = t.encode_utf16().chain(Some(0)).collect();
                        TextOutW(self.mem_dc, text_x, text_y, &wide);
                        SelectObject(self.mem_dc, old_font);
                    }
                } else {
                    let pen = CreatePen(PS_SOLID, bw, COLORREF(color_rgb));
                    let old_pen = SelectObject(self.mem_dc, pen);
                    let rr = self.border_radius;
                    RoundRect(
                        self.mem_dc,
                        x + half,
                        y + half,
                        x + w - half,
                        y + h - half,
                        rr, rr,
                    );
                    SelectObject(self.mem_dc, old_pen);
                    let _ = DeleteObject(pen);
                }
            }

            let mut ptr = self.bits as *mut u32;
            let count = (self.width * self.height) as usize;
            for i in 0..count {
                if *ptr != 0 {
                    *ptr |= 0xFF000000;
                }
                ptr = ptr.add(1);
            }

            let mut size = SIZE {
                cx: self.width,
                cy: self.height,
            };
            let mut src_pt = POINT { x: 0, y: 0 };
            let mut dst_pt = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            UpdateLayeredWindow(
                self.hwnd,
                self.mem_dc,
                Some(&mut dst_pt),
                Some(&mut size),
                self.mem_dc,
                Some(&mut src_pt),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }

    pub fn set_position(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                x,
                y,
                width,
                height,
                SWP_SHOWWINDOW,
            );
        }
    }

    pub fn set_alpha(&self, alpha: u8) {
        unsafe {
            let mut size = SIZE {
                cx: self.width,
                cy: self.height,
            };
            let mut src_pt = POINT { x: 0, y: 0 };
            let mut dst_pt = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: alpha,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            UpdateLayeredWindow(
                self.hwnd,
                self.mem_dc,
                Some(&mut dst_pt),
                Some(&mut size),
                self.mem_dc,
                Some(&mut src_pt),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }
}

impl Drop for BorderOverlay {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.mem_dc, self.old_bitmap);
            DeleteObject(self.bitmap);
            DeleteObject(self.title_font);
            DeleteDC(self.mem_dc);
            DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn border_wnd_proc(
    hwnd: HWND,
    msg: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, _wparam, _lparam),
    }
}
