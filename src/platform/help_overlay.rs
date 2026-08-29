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

pub static mut HELP_PTR: *mut HelpOverlay = std::ptr::null_mut();

pub struct HelpOverlay {
    pub hwnd: HWND,
    width: i32,
    height: i32,
}

impl HelpOverlay {
    pub fn create() -> anyhow::Result<()> {
        unsafe {
            let hinstance = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(help_wnd_proc),
                hInstance: hinstance,
                lpszClassName: w!("UltraWMHelpOverlay"),
                hbrBackground: HBRUSH(GetStockObject(HOLLOW_BRUSH).0),
                ..Default::default()
            };
            RegisterClassW(&class);

            let primary = get_primary_monitor_size();
            let width = 700;
            let height = 500;
            let x = (primary.0 - width) / 2;
            let y = (primary.1 - height) / 2;

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                w!("UltraWMHelpOverlay"),
                w!("UltraWM Keybinds"),
                WS_POPUP | WS_VISIBLE,
                x, y, width, height,
                None, None, hinstance, None,
            )?;

            let help = Self { hwnd, width, height };
            let boxed = Box::new(help);
            let leaked = Box::leak(boxed);
            HELP_PTR = leaked as *mut HelpOverlay;

            // Render the initial content
            leaked.render();

            Ok(())
        }
    }

    pub fn dismiss(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }

    pub fn render(&self) {
        unsafe {
            let hdc_screen = GetDC(None);

            // Create a 32-bit ARGB bitmap for per-pixel alpha
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

            // Draw content into the bitmap using GDI
            let mem_dc = CreateCompatibleDC(hdc_screen);
            let old_bmp = SelectObject(mem_dc, dib);

            // Clear with transparent
            let _ = PatBlt(mem_dc, 0, 0, self.width, self.height, WHITENESS);

            self.draw_content(mem_dc);

            // Update the layered window with per-pixel alpha
            let mut pt_src = POINT { x: 0, y: 0 };
            let mut sz = SIZE { cx: self.width, cy: self.height };
            let mut pt_dst = POINT { x: 0, y: 0 };

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
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

    fn draw_content(&self, hdc: HDC) {
        unsafe {
            let width = self.width;
            let height = self.height;
            let radius = 12;

            // Draw rounded rectangle background with per-pixel alpha
            let _bg_color = 0xFF1E1E2E; // ABGR format
            let _r = (_bg_color >> 16) & 0xFF;
            let _g = (_bg_color >> 8) & 0xFF;
            let _b = _bg_color & 0xFF;
            let _a = (_bg_color >> 24) & 0xFF;

            // Draw rounded rect using GDI

            // Use RoundRect for the background
            RoundRect(hdc, 0, 0, width, height, radius * 2, radius * 2);

            // Set the overall alpha using a memory DC trick
            // We'll use SetLayeredWindowAttributes approach for the overall window
            // but draw rounded corners by masking

            // Re-draw with proper alpha
            // Clear the bitmap
            let _ = PatBlt(hdc, 0, 0, width, height, WHITENESS);

            // Draw the rounded rectangle using GDI
            let hbrush = CreateSolidBrush(COLORREF(0x001E1E2E));
            let hpen = CreatePen(PS_SOLID, 0, COLORREF(0x001E1E2E));
            let old_brush = SelectObject(hdc, hbrush);
            let old_pen = SelectObject(hdc, hpen);

            RoundRect(hdc, 0, 0, width, height, radius * 2, radius * 2);

            // Draw a shadow border
            let shadow_pen = CreatePen(PS_SOLID, 1, COLORREF(0x40313150));
            let _ = SelectObject(hdc, shadow_pen);
            let _ = SelectObject(hdc, GetStockObject(NULL_BRUSH));
            RoundRect(hdc, 0, 0, width, height, radius * 2, radius * 2);

            let _ = SelectObject(hdc, old_brush);
            let _ = SelectObject(hdc, old_pen);
            let _ = DeleteObject(hbrush);
            let _ = DeleteObject(hpen);
            let _ = DeleteObject(shadow_pen);

            // Draw title
            let title = "UltraWM Keybinds";
            let title_w: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
            let mut title_rect = RECT {
                left: 20,
                top: 15,
                right: 680,
                bottom: 50,
            };
            SetTextColor(hdc, COLORREF(0x00F4D6F4)); // ABGR: CDD6F4
            SetBkMode(hdc, TRANSPARENT);
            let title_font = CreateFontW(
                24, 0, 0, 0, FW_BOLD.0 as i32,
                0u32, 0u32, 0u32,
                DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32, DEFAULT_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                w!("Segoe UI"),
            );
            let old_font = SelectObject(hdc, title_font);
            DrawTextW(
                hdc,
                &mut title_w.clone(),
                &mut title_rect,
                DT_SINGLELINE | DT_LEFT,
            );

            // Get keybinds
            let kb = crate::platform::keyboard::PLATFORM_PTR;
            let keybinds = if !kb.is_null() {
                let platform = &*kb;
                platform.keybinds
            } else {
                crate::platform::keybinds::ParsedKeybinds {
                    focus_left: 0, focus_right: 0, focus_up: 0, focus_down: 0,
                    move_left: 0, move_right: 0, move_up: 0, move_down: 0,
                    pan_left: 0, pan_right: 0, pan_up: 0, pan_down: 0,
                    grow_width: 0, shrink_width: 0, grow_height: 0, shrink_height: 0,
                    fullscreen: 0, close: 0, float: 0, sticky: 0,
                    theme_next: 0, theme_prev: 0, theme_picker: 0,
                    launcher: 0, window_search: 0,
                }
            };

            // Set normal font for keybinds
            let normal_font = CreateFontW(
                16, 0, 0, 0, FW_NORMAL.0 as i32,
                0u32, 0u32, 0u32,
                DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32, DEFAULT_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                w!("Segoe UI"),
            );
            let _ = SelectObject(hdc, normal_font);

            // Helper to draw a keybind line
            let draw_line = |hdc: HDC, label: &str, key: u32, y: &mut i32| {
                let key_name = vk_to_string(key);
                let display = format!("{:20} {}", label, key_name);
                let text_w: Vec<u16> = display.encode_utf16().chain(Some(0)).collect();
                let mut text_rect = RECT {
                    left: 30,
                    top: *y,
                    right: 670,
                    bottom: *y + 22,
                };
                SetTextColor(hdc, COLORREF(0x00F4D6F4));
                SetBkMode(hdc, TRANSPARENT);
                DrawTextW(
                    hdc,
                    &mut text_w.clone(),
                    &mut text_rect,
                    DT_SINGLELINE | DT_LEFT,
                );
                *y += 22;
            };

            // Draw section headers
            let section_font = CreateFontW(
                16, 0, 0, 0, FW_BOLD.0 as i32,
                0u32, 0u32, 0u32,
                DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32, DEFAULT_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                w!("Segoe UI"),
            );
            let _ = SelectObject(hdc, section_font);

            let draw_section = |hdc: HDC, text: &str, y: &mut i32| {
                let text_w: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
                let mut text_rect = RECT {
                    left: 25,
                    top: *y,
                    right: 680,
                    bottom: *y + 20,
                };
                SetTextColor(hdc, COLORREF(0x00B4B4E0)); // ABGR: E0B4B4
                SetBkMode(hdc, TRANSPARENT);
                DrawTextW(
                    hdc,
                    &mut text_w.clone(),
                    &mut text_rect,
                    DT_SINGLELINE | DT_LEFT,
                );
                *y += 20;
            };

            let mut y = 60;

            // Focus/Move
            draw_section(hdc, "FOCUS / MOVE", &mut y);
            let _ = SelectObject(hdc, normal_font);
            draw_line(hdc, "  Focus left:", keybinds.focus_left, &mut y);
            draw_line(hdc, "  Focus right:", keybinds.focus_right, &mut y);
            draw_line(hdc, "  Focus up:", keybinds.focus_up, &mut y);
            draw_line(hdc, "  Focus down:", keybinds.focus_down, &mut y);
            draw_line(hdc, "  Move left:", keybinds.move_left, &mut y);
            draw_line(hdc, "  Move right:", keybinds.move_right, &mut y);
            draw_line(hdc, "  Move up:", keybinds.move_up, &mut y);
            draw_line(hdc, "  Move down:", keybinds.move_down, &mut y);
            y += 8;

            // Pan/Resize
            let _ = SelectObject(hdc, section_font);
            draw_section(hdc, "PAN / RESIZE", &mut y);
            let _ = SelectObject(hdc, normal_font);
            draw_line(hdc, "  Pan left:", keybinds.pan_left, &mut y);
            draw_line(hdc, "  Pan right:", keybinds.pan_right, &mut y);
            draw_line(hdc, "  Grow width:", keybinds.grow_width, &mut y);
            draw_line(hdc, "  Shrink width:", keybinds.shrink_width, &mut y);
            draw_line(hdc, "  Grow height:", keybinds.grow_height, &mut y);
            draw_line(hdc, "  Shrink height:", keybinds.shrink_height, &mut y);
            y += 8;

            // Actions
            let _ = SelectObject(hdc, section_font);
            draw_section(hdc, "ACTIONS", &mut y);
            let _ = SelectObject(hdc, normal_font);
            draw_line(hdc, "  Fullscreen:", keybinds.fullscreen, &mut y);
            draw_line(hdc, "  Close window:", keybinds.close, &mut y);
            draw_line(hdc, "  Float/Unfloat:", keybinds.float, &mut y);
            draw_line(hdc, "  Sticky:", keybinds.sticky, &mut y);
            y += 8;

            // UI
            let _ = SelectObject(hdc, section_font);
            draw_section(hdc, "UI", &mut y);
            let _ = SelectObject(hdc, normal_font);
            draw_line(hdc, "  Launcher:", keybinds.launcher, &mut y);
            draw_line(hdc, "  Window search:", keybinds.window_search, &mut y);
            draw_line(hdc, "  Theme picker:", keybinds.theme_picker, &mut y);

            // Footer
            let footer = "Press Esc to close";
            let footer_w: Vec<u16> = footer.encode_utf16().chain(Some(0)).collect();
            let mut footer_rect = RECT {
                left: 20,
                top: 460,
                right: 680,
                bottom: 490,
            };
            let _ = SelectObject(hdc, normal_font);
            SetTextColor(hdc, COLORREF(0x00756C86)); // ABGR: 6C7586
            DrawTextW(
                hdc,
                &mut footer_w.clone(),
                &mut footer_rect,
                DT_SINGLELINE | DT_LEFT,
            );

            // Cleanup
            let _ = SelectObject(hdc, old_font);
            let _ = DeleteObject(title_font);
            let _ = DeleteObject(normal_font);
            let _ = DeleteObject(section_font);
        }
    }
}

impl Drop for HelpOverlay {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn help_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            // Skip paint - content is rendered via UpdateLayeredWindow
            LRESULT(0)
        }
        WM_SHOWWINDOW => {
            if wparam.0 == 1 {
                if let Some(h) = HELP_PTR.as_ref() {
                    h.render();
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_KEYDOWN => {
            if wparam.0 as u32 == VK_ESCAPE.0 as u32 {
                if let Some(h) = HELP_PTR.as_ref() {
                    h.dismiss();
                }
            }
            LRESULT(0)
        }
        WM_KILLFOCUS => {
            if let Some(h) = HELP_PTR.as_ref() {
                h.dismiss();
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

fn vk_to_string(vk: u32) -> &'static str {
    match vk {
        x if x == VK_LEFT.0 as u32 => "Left",
        x if x == VK_RIGHT.0 as u32 => "Right",
        x if x == VK_UP.0 as u32 => "Up",
        x if x == VK_DOWN.0 as u32 => "Down",
        x if x == VK_SPACE.0 as u32 => "Space",
        x if x == VK_ESCAPE.0 as u32 => "Esc",
        x if x == VK_RETURN.0 as u32 => "Enter",
        x if x == VK_TAB.0 as u32 => "Tab",
        x if x == VK_BACK.0 as u32 => "Back",
        x if x == VK_F1.0 as u32 => "F1",
        x if x == VK_F2.0 as u32 => "F2",
        x if x == VK_F3.0 as u32 => "F3",
        x if x == VK_F4.0 as u32 => "F4",
        x if x == VK_F5.0 as u32 => "F5",
        x if x == VK_F6.0 as u32 => "F6",
        x if x == VK_F7.0 as u32 => "F7",
        x if x == VK_F8.0 as u32 => "F8",
        x if x == VK_F9.0 as u32 => "F9",
        x if x == VK_F10.0 as u32 => "F10",
        x if x == VK_F11.0 as u32 => "F11",
        x if x == VK_F12.0 as u32 => "F12",
        _ => {
            if vk >= 0x30 && vk <= 0x39 {
                char::from_digit((vk - 0x30) as u32, 10)
                    .unwrap_or('?')
                    .to_string()
                    .leak()
            } else if vk >= 0x41 && vk <= 0x5A {
                char::from_u32(vk)
                    .unwrap_or('?')
                    .to_string()
                    .leak()
            } else {
                "?"
            }
        }
    }
}

fn get_primary_monitor_size() -> (i32, i32) {
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        (w, h)
    }
}
