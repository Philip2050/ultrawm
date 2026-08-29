use std::ptr;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
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
    pub width: i32,
    pub height: i32,
    pub themes: Vec<String>,
    pub current: usize,
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

            let width = 420i32;
            let height = 340i32;
            let picker = Self {
                hwnd: HWND(0),
                width,
                height,
                themes: themes.clone(),
                current,
            };

            let boxed = Box::new(picker);
            let leaked = Box::leak(boxed);
            let ptr = leaked as *mut ThemePicker;

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(WS_EX_TOPMOST.0 | WS_EX_LAYERED.0),
                w!("UltraWMThemePicker"),
                w!("UltraWM Theme Picker"),
                WINDOW_STYLE(WS_POPUP.0),
                0, 0, width, height,
                None, None, hinstance,
                Some(ptr as *const _),
            )?;

            (*ptr).hwnd = hwnd;
            THEME_PICKER_PTR = ptr;

            // Per-pixel alpha render
            let _ = render(&*ptr);

            // Show centered on primary monitor
            let _ = SetWindowPos(hwnd, HWND_TOP,
                (GetSystemMetrics(SM_CXSCREEN) - width) / 2,
                (GetSystemMetrics(SM_CYSCREEN) - height) / 2,
                0, 0, SWP_NOSIZE | SWP_SHOWWINDOW);
            SetFocus(hwnd);

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

unsafe fn render(picker: &ThemePicker) -> Result<()> {
    use std::ptr;

    let w = picker.width;
    let h = picker.height;
    let hdc_screen = GetDC(HWND(ptr::null_mut()));
    let hdc_mem = CreateCompatibleDC(hdc_screen);

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [Default::default(); 1],
    };

    let mut pixels: Vec<u8> = vec![0u8; (w * h * 4) as usize];
    let mut dib = HBITMAP(0);
    let mut pv = &mut pixels[..] as *mut _ as *mut _;
    let _ = CreateDIBSection(hdc_mem, &mut bmi, DIB_RGB_COLORS, &mut pv, None, 0);
    let old_bmp = SelectObject(hdc_mem, dib);

    // Background — dark with rounded corners
    draw_round_rect(&mut pixels, w as u32, h as u32, 0xFF1E1E2E, 0xCC1E1E2E, 16);
    // Header
    draw_round_rect(&mut pixels, w as u32, h as u32, 0xFF313244, 0xFF313244, 16);
    for y in 32..44 {
        for x in 8..(w - 8) {
            set_pixel(&mut pixels, w as u32, h as u32, x, y, 0xFF313244);
        }
    }

    // Theme entries
    let start_y = 52;
    let item_h = 56;
    let item_w = w - 32;
    for (i, theme) in picker.themes.iter().enumerate() {
        let y = start_y + i as i32 * item_h;
        let is_selected = i == picker.current;

        if is_selected {
            // Selected highlight
            draw_round_rect(&mut pixels, w as u32, h as u32, 0xFFCBA6F7, 0x33CBA6F7, 8);
            for dy in 0..item_h {
                for dx in 8..(8 + item_w) {
                    let px = 16 + dx;
                    let py = y + dy;
                    if px > 8 && px < w - 8 && py > y && py < y + item_h - 1 {
                        if (py - y).abs() <= 4 || (py - (y + item_h - 1)).abs() <= 4 {
                            set_pixel(&mut pixels, w as u32, h as u32, px, py, 0xFFCBA6F7);
                        } else if (px - 8).abs() <= 4 || (px - (8 + item_w)).abs() <= 4 {
                            set_pixel(&mut pixels, w as u32, h as u32, px, py, 0xFFCBA6F7);
                        }
                    }
                }
            }
        }

        // Color preview dot
        let cx = 28;
        let cy = y + item_h / 2;
        let cr = 10;
        for dy in -(cr)..=cr {
            for dx in -(cr)..=cr {
                if dx * dx + dy * dy <= cr * cr {
                    set_pixel(&mut pixels, w as u32, h as u32, cx + dx, cy + dy, 0xFFF38BA8);
                }
            }
        }

        // Theme name
        let text = format!("{} {}", if is_selected { ">" } else { " " }, theme);
        let text_w: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let mut rect = RECT {
            left: 50,
            top: y + 6,
            right: w - 20,
            bottom: y + item_h,
        };
        let mut hfont = create_font(14, FW_NORMAL.0 as i32);
        let old_font = SelectObject(hdc_mem, hfont);
        let _ = DrawTextW(hdc_mem, &mut text_w.clone(), &mut rect,
            DT_SINGLELINE | DT_VCENTER | DT_LEFT);
        DeleteObject(SelectObject(hdc_mem, old_font));
    }

    // Apply via UpdateLayeredWindow
    let mut pt_src = POINT { x: 0, y: 0 };
    let mut blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER.0 as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA.0 as u8,
    };
    let mut size = SIZE { cx: w, cy: h };
    let _ = UpdateLayeredWindow(hwnd, hdc_screen, None, Some(&mut size),
        hdc_mem, Some(&pt_src), COLORREF(0), Some(&mut blend), ULW_ALPHA);

    SelectObject(hdc_mem, old_bmp);
    DeleteObject(dib);
    DeleteDC(hdc_mem);
    ReleaseDC(HWND(ptr::null_mut()), hdc_screen);
    Ok(())
}

fn draw_round_rect(pixels: &mut [u8], w: u32, h: u32, fill: u32, border: u32, r: i32) {
    let bg_r = ((fill >> 24) & 0xFF) as u8;
    let bg_g = ((fill >> 16) & 0xFF) as u8;
    let bg_b = (fill & 0xFF) as u8;
    let bg_a = ((fill >> 24) & 0xFF) as u8;
    if bg_a == 0 { return; }
    let br = ((border >> 24) & 0xFF) as u8;
    let bg2_r = ((border >> 16) & 0xFF) as u8;
    let bg2_g = ((border >> 8) & 0xFF) as u8;
    let bg2_b = (border & 0xFF) as u8;

    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let is_border = x < r || x >= w as i32 - r || y < r || y >= h as i32 - r;
            let in_corner = (x < r && y < r)
                || (x >= w as i32 - r && y < r)
                || (x < r && y >= h as i32 - r)
                || (x >= w as i32 - r && y >= h as i32 - r);
            if in_corner {
                let cx = if x < r { r } else { w as i32 - r - 1 };
                let cy = if y < r { r } else { h as i32 - r - 1 };
                let dx = x - cx;
                let dy = y - cy;
                if dx * dx + dy * dy > r * r {
                    let idx = ((y * w as i32 + x) * 4) as usize;
                    if is_border {
                        pixels[idx] = br;
                        pixels[idx + 1] = bg2_r;
                        pixels[idx + 2] = bg2_g;
                        pixels[idx + 3] = bg2_b;
                    } else {
                        pixels[idx] = bg_a;
                        pixels[idx + 1] = bg_r;
                        pixels[idx + 2] = bg_g;
                        pixels[idx + 3] = bg_b;
                    }
                }
            }
        }
    }
}

fn set_pixel(pixels: &mut [u8], w: u32, h: u32, x: i32, y: i32, color: u32) {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 { return; }
    let idx = ((y * w as i32 + x) * 4) as usize;
    pixels[idx] = ((color >> 24) & 0xFF) as u8;
    pixels[idx + 1] = ((color >> 16) & 0xFF) as u8;
    pixels[idx + 2] = ((color >> 8) & 0xFF) as u8;
    pixels[idx + 3] = (color & 0xFF) as u8;
}

fn create_font(height: i32, weight: i32) -> HFONT {
    unsafe {
        CreateFontW(
            height, 0, 0, 0, weight,
            0, 0, 0, DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            DEFAULT_QUALITY.0 as u32,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
            w!("Segoe UI"),
        )
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
                        // Theme selection handled via double-click on list
                    }
                    x if x == VK_UP.0 as u32 => {
                        // Navigation handled by listbox
                    }
                    x if x == VK_DOWN.0 as u32 => {
                        // Navigation handled by listbox
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
