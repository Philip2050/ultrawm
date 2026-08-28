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

pub static mut BORDER_PTR: *mut BorderOverlay = null_mut();

pub struct BorderOverlay {
    pub hwnd: HWND,
    pub border_width: i32,
    pub border_radius: i32,
    pub overview_positions: Vec<(i32, i32, i32, i32, HWND)>,
    pub tile_rects: Vec<(i32, i32, i32, i32, HWND)>,
    width: i32,
    height: i32,
    mem_dc: HDC,
    bitmap: HBITMAP,
    old_bitmap: HGDIOBJ,
    bits: *mut u8,
    title_font: HFONT,
    resize_target: Option<(HWND, i32, i32, i32, i32)>,
    resize_edge: u8,
}

pub const WM_OVERVIEW_CLICK: u32 = WM_USER + 0x100;

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
                overview_positions: Vec::new(),
                tile_rects: Vec::new(),
                width,
                height,
                mem_dc,
                bitmap,
                old_bitmap,
                bits: bits as *mut u8,
                title_font,
                resize_target: None,
                resize_edge: 0,
            })
        }
    }

    pub fn set_transparent(&self, transparent: bool) {
        unsafe {
            let mut ex_style = GetWindowLongW(self.hwnd, GWL_EXSTYLE) as u32;
            if transparent {
                ex_style |= WS_EX_TRANSPARENT.0;
            } else {
                ex_style &= !WS_EX_TRANSPARENT.0;
            }
            let _ = SetWindowLongW(self.hwnd, GWL_EXSTYLE, ex_style as i32);
        }
    }

    pub fn update(&mut self, rects: &[(i32, i32, i32, i32, u32, bool, bool, Option<String>)]) {
        unsafe {
            std::ptr::write_bytes(self.bits, 0, (self.width * self.height * 4) as usize);

            for &(x, y, w, h, color_rgb, focused, floating, ref title) in rects {
                let bw = self.border_width;
                let half = bw / 2;
                let pen_color = if floating { 0xFF4488FF } else { color_rgb };

                if focused && !floating {
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

                    // Window title with background
                    if let Some(ref t) = title {
                        let old_font = SelectObject(self.mem_dc, self.title_font);
                        SetBkMode(self.mem_dc, TRANSPARENT);
                        SetTextColor(self.mem_dc, COLORREF(0xFFFFFF));

                        let text_y = y + half + 2;
                        let text_x = x + half + 6;
                        let mut wide: Vec<u16> = t.encode_utf16().chain(Some(0)).collect();

                        // Measure text width
                        let mut sz = SIZE { cx: 0, cy: 0 };
                        let _ = GetTextExtentPoint32W(self.mem_dc, &wide, &mut sz);
                        let text_w = sz.cx + 12;
                        let text_h = 16;

                        // Draw title background
                        let bg_brush = CreateSolidBrush(COLORREF(color_rgb & 0xFF333333));
                        let old_brush = SelectObject(self.mem_dc, bg_brush);
                        let _ = PatBlt(self.mem_dc, text_x, text_y, text_w, text_h, PATCOPY);
                        SelectObject(self.mem_dc, old_brush);
                        let _ = DeleteObject(bg_brush);

                        // Draw title text
                        TextOutW(self.mem_dc, text_x + 6, text_y + 1, &wide);
                        SelectObject(self.mem_dc, old_font);
                    }
                } else if floating {
                    // Floating window: dashed blue border
                    let pen = CreatePen(PS_DASH, bw, COLORREF(0xFF4488FF));
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

                    // "FLOATING" label
                    let label: Vec<u16> = "FLOATING".encode_utf16().chain(Some(0)).collect();
                    let old_font = SelectObject(self.mem_dc, self.title_font);
                    SetBkMode(self.mem_dc, TRANSPARENT);
                    SetTextColor(self.mem_dc, COLORREF(0xFF4488FF));
                    let text_y = y + half + 2;
                    let text_x = x + half + 6;
                    TextOutW(self.mem_dc, text_x, text_y, &label);
                    SelectObject(self.mem_dc, old_font);
                } else {
                    let pen = CreatePen(PS_SOLID, bw, COLORREF(pen_color));
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

    pub fn set_tile_rects(&mut self, rects: Vec<(i32, i32, i32, i32, HWND)>) {
        self.tile_rects = rects;
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
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let ptr = BORDER_PTR;
            if !ptr.is_null() {
                let overlay = &mut *ptr;
                let x = (lparam.0 & 0xFFFF) as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i32;

                // If resizing, apply the resize
                if let Some((target_hwnd, sx, sy, sw, sh)) = overlay.resize_target {
                    let dx = x - sx;
                    let dy = y - sy;
                    let edge = overlay.resize_edge;
                    let mut nx = sx;
                    let mut ny = sy;
                    let mut nw = sw;
                    let mut nh = sh;

                    if edge & 1 != 0 { nw = (sw + dx).max(100); }
                    if edge & 2 != 0 { nh = (sh + dy).max(100); }
                    if edge & 4 != 0 { nx = sx + dx; nw = (sw - dx).max(100); }
                    if edge & 8 != 0 { ny = sy + dy; nh = (sh - dy).max(100); }

                    let _ = SetWindowPos(target_hwnd, HWND(null_mut()), nx, ny, nw, nh, SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
                    return LRESULT(0);
                }

                // Check proximity to tiled window edges for cursor
                let edge_size = 6;
                let mut cursor = IDC_ARROW;
                for &(rx, ry, rw, rh, _) in &overlay.tile_rects {
                    let near_left = x >= rx && x < rx + edge_size && y >= ry && y < ry + rh;
                    let near_right = x >= rx + rw - edge_size && x < rx + rw && y >= ry && y < ry + rh;
                    let near_top = y >= ry && y < ry + edge_size && x >= rx && x < rx + rw;
                    let near_bottom = y >= ry + rh - edge_size && y < ry + rh && x >= rx && x < rx + rw;

                    if (near_left || near_right) && (near_top || near_bottom) { cursor = IDC_SIZEALL; break; }
                    if near_left || near_right { cursor = IDC_SIZEWE; }
                    if near_top || near_bottom { cursor = IDC_SIZENS; }
                }
                let hcursor = LoadCursorW(HINSTANCE(null_mut()), cursor);
                if let Ok(c) = hcursor {
                    SetCursor(c);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let ptr = BORDER_PTR;
            if !ptr.is_null() {
                let overlay = &mut *ptr;
                let x = (lparam.0 & 0xFFFF) as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i32;

                // Check if near a tiled window edge for resize
                let edge_size = 6;
                for &(rx, ry, rw, rh, win_hwnd) in &overlay.tile_rects {
                    let near_left = x >= rx && x < rx + edge_size && y >= ry && y < ry + rh;
                    let near_right = x >= rx + rw - edge_size && x < rx + rw && y >= ry && y < ry + rh;
                    let near_top = y >= ry && y < ry + edge_size && x >= rx && x < rx + rw;
                    let near_bottom = y >= ry + rh - edge_size && y < ry + rh && x >= rx && x < rx + rw;

                    if near_left || near_right || near_top || near_bottom {
                        let mut edge: u8 = 0;
                        if near_left { edge |= 1; }
                        if near_right { edge |= 2; }
                        if near_top { edge |= 4; }
                        if near_bottom { edge |= 8; }
                        overlay.resize_target = Some((win_hwnd, x, y, rw, rh));
                        overlay.resize_edge = edge;
                        return LRESULT(1);
                    }
                }

                // Overview click
                for &(rx, ry, rw, rh, hwnd_val) in &overlay.overview_positions {
                    if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                        let _ = PostMessageW(hwnd, WM_OVERVIEW_CLICK, WPARAM(hwnd_val.0 as usize), LPARAM(0));
                        return LRESULT(0);
                    }
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let ptr = BORDER_PTR;
            if !ptr.is_null() {
                let overlay = &mut *ptr;
                if overlay.resize_target.is_some() {
                    overlay.resize_target = None;
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
