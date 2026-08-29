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
    move_source: Option<HWND>,
    drag_start: Option<(i32, i32, HWND)>,
    drag_active: bool,
    drag_ghost: Option<(i32, i32, i32, i32)>,
}

pub const WM_OVERVIEW_CLICK: u32 = WM_USER + 0x100;
pub const WM_SWAP_WINDOWS: u32 = WM_USER + 0x101;
pub const WM_DRAG_MOVE: u32 = WM_USER + 0x102;
pub const WM_EDGE_TILE: u32 = WM_USER + 0x103;

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
                move_source: None,
                drag_start: None,
                drag_active: false,
                drag_ghost: None,
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

    pub fn update(&mut self, rects: &[(i32, i32, i32, i32, u32, bool, bool, Option<String>, i32)]) {
        unsafe {
            std::ptr::write_bytes(self.bits, 0, (self.width * self.height * 4) as usize);

            for &(x, y, w, h, color_rgb, focused, floating, ref title, bw) in rects {
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

                // Window title for all windows
                if let Some(ref t) = title {
                    let old_font = SelectObject(self.mem_dc, self.title_font);
                    SetBkMode(self.mem_dc, TRANSPARENT);

                    let text_y = y + half + 2;
                    let text_x = x + half + 6;
                    let mut wide: Vec<u16> = t.encode_utf16().chain(Some(0)).collect();

                    // Measure text width
                    let mut sz = SIZE { cx: 0, cy: 0 };
                    let _ = GetTextExtentPoint32W(self.mem_dc, &wide, &mut sz);
                    let text_w = sz.cx + 12;
                    let text_h = 16;

                    if focused && !floating {
                        // Focused window: accent background, white text
                        SetTextColor(self.mem_dc, COLORREF(0xFFFFFF));
                        let bg_brush = CreateSolidBrush(COLORREF(color_rgb & 0xFF333333));
                        let old_brush = SelectObject(self.mem_dc, bg_brush);
                        let _ = PatBlt(self.mem_dc, text_x, text_y, text_w, text_h, PATCOPY);
                        SelectObject(self.mem_dc, old_brush);
                        let _ = DeleteObject(bg_brush);
                    } else {
                        // Unfocused: dimmed text on semi-transparent dark background
                        let dim_color = if floating { 0xFF4488FF } else { color_dim(color_rgb, 0.5) };
                        SetTextColor(self.mem_dc, COLORREF(dim_color));
                        let bg_brush = CreateSolidBrush(COLORREF(0x40000000));
                        let old_brush = SelectObject(self.mem_dc, bg_brush);
                        let _ = PatBlt(self.mem_dc, text_x, text_y, text_w, text_h, PATCOPY);
                        SelectObject(self.mem_dc, old_brush);
                        let _ = DeleteObject(bg_brush);
                    }

                    TextOutW(self.mem_dc, text_x + 6, text_y + 1, &wide);
                    SelectObject(self.mem_dc, old_font);
                }
            }

            // Draw drag ghost (highlighted target cell)
            if let Some((gx, gy, gw, gh)) = self.drag_ghost {
                let ghost_pen = CreatePen(PS_SOLID, 3, COLORREF(0xFF00FF00));
                let old_pen = SelectObject(self.mem_dc, ghost_pen);
                let ghost_brush = CreateSolidBrush(COLORREF(0x3000FF00));
                let old_brush = SelectObject(self.mem_dc, ghost_brush);
                Rectangle(self.mem_dc, gx, gy, gx + gw, gy + gh);
                SelectObject(self.mem_dc, old_brush);
                DeleteObject(ghost_brush);
                SelectObject(self.mem_dc, old_pen);
                DeleteObject(ghost_pen);
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

                    // Enforce min/max constraints from rules
                    let tgt_wrapper = crate::platform::HWnd(target_hwnd);
                    unsafe {
                        let ptr = crate::platform::keyboard::PLATFORM_PTR;
                        if !ptr.is_null() {
                            let platform = &*ptr;
                            if let Some(info) = platform.windows.get(&tgt_wrapper) {
                                if let Some(min_w) = info.min_width { if nw < min_w as i32 { nw = min_w as i32; } }
                                if let Some(min_h) = info.min_height { if nh < min_h as i32 { nh = min_h as i32; } }
                                if let Some(max_w) = info.max_width { if nw > max_w as i32 { nw = max_w as i32; } }
                                if let Some(max_h) = info.max_height { if nh > max_h as i32 { nh = max_h as i32; } }
                            }
                        }
                    }

                    let _ = SetWindowPos(target_hwnd, HWND(null_mut()), nx, ny, nw, nh, SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
                    return LRESULT(0);
                }

                // Check proximity to tiled window edges for cursor
                let edge_size = 6;
                let mut cursor = IDC_ARROW;
                let mut on_window = false;
                for &(rx, ry, rw, rh, _) in &overlay.tile_rects {
                    let near_left = x >= rx && x < rx + edge_size && y >= ry && y < ry + rh;
                    let near_right = x >= rx + rw - edge_size && x < rx + rw && y >= ry && y < ry + rh;
                    let near_top = y >= ry && y < ry + edge_size && x >= rx && x < rx + rw;
                    let near_bottom = y >= ry + rh - edge_size && y < ry + rh && x >= rx && x < rx + rw;

                    if (near_left || near_right) && (near_top || near_bottom) { cursor = IDC_SIZEALL; break; }
                    if near_left || near_right { cursor = IDC_SIZEWE; }
                    if near_top || near_bottom { cursor = IDC_SIZENS; }

                    if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                        on_window = true;
                    }
                }
                if overlay.drag_active {
                    cursor = IDC_HAND;
                } else if cursor == IDC_ARROW && on_window {
                    cursor = IDC_HAND;
                }
                let hcursor = LoadCursorW(HINSTANCE(null_mut()), cursor);
                if let Ok(c) = hcursor {
                    SetCursor(c);
                }

                // Track drag movement
                if let Some((sx, sy, _src)) = overlay.drag_start {
                    let dx = x - sx;
                    let dy = y - sy;
                    if (dx * dx + dy * dy) > 25 {
                        overlay.drag_active = true;
                        // Find target cell under cursor
                        overlay.drag_ghost = None;
                        for &(rx, ry, rw, rh, _) in &overlay.tile_rects {
                            if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                                overlay.drag_ghost = Some((rx, ry, rw, rh));
                                break;
                            }
                        }
                    }
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

                // Check if on a tiled window for move/swap/drag
                for &(rx, ry, rw, rh, win_hwnd) in &overlay.tile_rects {
                    if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                        overlay.drag_start = Some((x, y, win_hwnd));
                        overlay.move_source = Some(win_hwnd);
                        return LRESULT(1);
                    }
                }

                // Clicked empty space — cancel move
                overlay.move_source = None;

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
                // Handle drag move
                if overlay.drag_active {
                    if let Some((_sx, _sy, src_hwnd)) = overlay.drag_start {
                        let cx = ((lparam.0 & 0xFFFF) as i32);
                        let cy = (((lparam.0 >> 16) & 0xFFFF) as i32);
                        let w = overlay.width;
                        let h = overlay.height;
                        let edge = 20;

                        // Check edge tiling zones (mode codes: 0=maximize, 1=top-left, 2=top-right, 3=bottom-left, 4=bottom-right, 5=left, 6=right, 7=bottom)
                        let mode_code = if cx < edge && cy < edge {
                            Some(1)
                        } else if cx > w - edge && cy < edge {
                            Some(2)
                        } else if cx < edge && cy > h - edge {
                            Some(3)
                        } else if cx > w - edge && cy > h - edge {
                            Some(4)
                        } else if cy < edge {
                            Some(0)
                        } else if cx < edge {
                            Some(5)
                        } else if cx > w - edge {
                            Some(6)
                        } else if cy > h - edge {
                            Some(7)
                        } else {
                            None
                        };

                        if let Some(code) = mode_code {
                            let _ = PostMessageW(hwnd, WM_EDGE_TILE,
                                WPARAM(src_hwnd.0 as usize),
                                LPARAM(code as isize));
                        } else if let Some((tx, ty, tw, th)) = overlay.drag_ghost {
                            for &(rx, ry, rw, rh, tgt_hwnd) in &overlay.tile_rects {
                                if rx == tx && ry == ty && rw == tw && rh == th {
                                    let _ = PostMessageW(hwnd, WM_DRAG_MOVE,
                                        WPARAM(src_hwnd.0 as usize),
                                        LPARAM(tgt_hwnd.0 as isize));
                                    break;
                                }
                            }
                        }
                    }
                    overlay.drag_active = false;
                    overlay.drag_start = None;
                    overlay.drag_ghost = None;
                } else {
                    // Quick click without drag — swap windows
                    if let Some(src) = overlay.move_source {
                        for &(_rx, _ry, _rw, _rh, win_hwnd) in &overlay.tile_rects {
                            let cx = ((lparam.0 & 0xFFFF) as i32);
                            let cy = (((lparam.0 >> 16) & 0xFFFF) as i32);
                            for &(rx, ry, rw, rh, wh) in &overlay.tile_rects {
                                if cx >= rx && cx < rx + rw && cy >= ry && cy < ry + rh {
                                    if src != wh {
                                        let _ = PostMessageW(hwnd, WM_SWAP_WINDOWS,
                                            WPARAM(src.0 as usize),
                                            LPARAM(wh.0 as isize));
                                    }
                                    break;
                                }
                            }
                        }
                        overlay.move_source = None;
                    }
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
