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

pub static mut HELP_PTR: *mut HelpOverlay = std::ptr::null_mut();

pub struct HelpOverlay {
    pub hwnd: HWND,
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

            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 240, LWA_ALPHA);

            let help = Self { hwnd };
            let boxed = Box::new(help);
            let leaked = Box::leak(boxed);
            HELP_PTR = leaked as *mut HelpOverlay;

            Ok(())
        }
    }

    pub fn dismiss(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
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
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            // Draw background
            let bg_brush = CreateSolidBrush(COLORREF(0xFF1E1E2E));
            let rect = RECT {
                left: 0,
                top: 0,
                right: 700,
                bottom: 500,
            };
            FillRect(hdc, &rect, bg_brush);
            DeleteObject(bg_brush);

            // Draw title
            let title = "UltraWM Keybinds";
            let title_w: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
            let mut title_rect = RECT {
                left: 20,
                top: 15,
                right: 680,
                bottom: 50,
            };
            let _ = SetTextColor(hdc, COLORREF(0xFFCDD6F4));
            let _ = SetBkMode(hdc, TRANSPARENT);
            let _ = DrawTextW(
                hdc,
                &mut title_w.clone(),
                &mut title_rect,
                DT_SINGLELINE | DT_LEFT,
            );

            // Get keybinds from config
            let kb = crate::platform::keyboard::PLATFORM_PTR;
            let keybinds = if !kb.is_null() {
                let platform = &*kb;
                platform.keybinds
            } else {
                crate::platform::keybinds::ParsedKeybinds {
                    focus_left: 0,
                    focus_right: 0,
                    focus_up: 0,
                    focus_down: 0,
                    move_left: 0,
                    move_right: 0,
                    move_up: 0,
                    move_down: 0,
                    pan_left: 0,
                    pan_right: 0,
                    pan_up: 0,
                    pan_down: 0,
                    grow_width: 0,
                    shrink_width: 0,
                    grow_height: 0,
                    shrink_height: 0,
                    fullscreen: 0,
                    close: 0,
                    float: 0,
                    sticky: 0,
                    theme_next: 0,
                    theme_prev: 0,
                    theme_picker: 0,
                    launcher: 0,
                    window_search: 0,
                }
            };

            // Draw keybinds
            let mut y = 60;
            let line_height = 28;

            // Helper to draw a keybind line
            let draw_line = |hdc: HDC, text: &str, key: u32, y: &mut i32| {
                let key_name = vk_to_string(key);
                let display = format!("{:20} {}", text, key_name);
                let text_w: Vec<u16> = display.encode_utf16().chain(Some(0)).collect();
                let mut text_rect = RECT {
                    left: 30,
                    top: *y,
                    right: 670,
                    bottom: *y + line_height,
                };
                let _ = SetTextColor(hdc, COLORREF(0xFFCDD6F4));
                let _ = DrawTextW(
                    hdc,
                    &mut text_w.clone(),
                    &mut text_rect,
                    DT_SINGLELINE | DT_LEFT,
                );
                *y += line_height;
            };

            // Focus/Move
            draw_line(hdc, "Focus left:", keybinds.focus_left, &mut y);
            draw_line(hdc, "Focus right:", keybinds.focus_right, &mut y);
            draw_line(hdc, "Focus up:", keybinds.focus_up, &mut y);
            draw_line(hdc, "Focus down:", keybinds.focus_down, &mut y);
            draw_line(hdc, "Move left:", keybinds.move_left, &mut y);
            draw_line(hdc, "Move right:", keybinds.move_right, &mut y);
            draw_line(hdc, "Move up:", keybinds.move_up, &mut y);
            draw_line(hdc, "Move down:", keybinds.move_down, &mut y);
            y += 10;

            // Pan/Resize
            draw_line(hdc, "Pan left:", keybinds.pan_left, &mut y);
            draw_line(hdc, "Pan right:", keybinds.pan_right, &mut y);
            draw_line(hdc, "Grow width:", keybinds.grow_width, &mut y);
            draw_line(hdc, "Shrink width:", keybinds.shrink_width, &mut y);
            draw_line(hdc, "Grow height:", keybinds.grow_height, &mut y);
            draw_line(hdc, "Shrink height:", keybinds.shrink_height, &mut y);
            y += 10;

            // Actions
            draw_line(hdc, "Fullscreen:", keybinds.fullscreen, &mut y);
            draw_line(hdc, "Close window:", keybinds.close, &mut y);
            draw_line(hdc, "Float/Unfloat:", keybinds.float, &mut y);
            draw_line(hdc, "Sticky:", keybinds.sticky, &mut y);
            y += 10;

            // UI
            draw_line(hdc, "Launcher:", keybinds.launcher, &mut y);
            draw_line(hdc, "Window search:", keybinds.window_search, &mut y);
            draw_line(hdc, "Theme picker:", keybinds.theme_picker, &mut y);

            // Footer
            let footer = "Press Esc to close";
            let footer_w: Vec<u16> = footer.encode_utf16().chain(Some(0)).collect();
            let mut footer_rect = RECT {
                left: 20,
                top: 460,
                right: 680,
                bottom: 490,
            };
            let _ = SetTextColor(hdc, COLORREF(0xFF6C7086));
            let _ = DrawTextW(
                hdc,
                &mut footer_w.clone(),
                &mut footer_rect,
                DT_SINGLELINE | DT_LEFT,
            );

            let _ = EndPaint(hwnd, &mut ps);
            LRESULT(0)
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
            // Try to convert to char
            if vk >= 0x30 && vk <= 0x39 {
                // 0-9
                char::from_digit((vk - 0x30) as u32, 10)
                    .unwrap_or('?')
                    .to_string()
                    .leak()
            } else if vk >= 0x41 && vk <= 0x5A {
                // A-Z
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
