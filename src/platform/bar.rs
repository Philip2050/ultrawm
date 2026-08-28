use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        Media::Audio::*,
        System::{LibraryLoader::*, Power::*},
        UI::WindowsAndMessaging::*,
    },
};

pub const WM_BAR_WORKSPACE_CLICK: u32 = WM_APP + 0x100;

struct BarState {
    workspaces: Vec<String>,
    active_workspace: usize,
    title: String,
    title_color: u32,
    bg_color: u32,
    fg_color: u32,
    clock: String,
    battery: u32,
    volume: u32,
    corner_radius: i32,
    show_workspaces: bool,
    show_clock: bool,
    show_volume: bool,
    show_battery: bool,
    workspace_count: usize,
}

pub struct AppBar {
    pub hwnd: HWND,
    pub height: i32,
    pub width: i32,
}

impl AppBar {
    pub fn create(
        width: i32,
        height: i32,
        bg_color: u32,
        fg_color: u32,
        transparency: f32,
        workspace_count: usize,
        show_workspaces: bool,
        show_clock: bool,
        show_volume: bool,
        show_battery: bool,
    ) -> anyhow::Result<Self> {
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
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                w!("UltraWMAppBar"),
                w!("UltraWM Bar"),
                WS_POPUP | WS_VISIBLE,
                0, 0, width, height,
                None, None, hinstance, None,
            )?;

            let alpha = (transparency * 255.0).clamp(0.0, 255.0) as u8;
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);

            // Apply rounded corners
            let rgn = CreateRoundRectRgn(0, 0, width, height, 10, 10);
            if !rgn.is_invalid() {
                let _ = SetWindowRgn(hwnd, rgn, false);
            }

            let workspaces: Vec<String> = (1..=workspace_count).map(|i| i.to_string()).collect();
            let state = BarState {
                workspaces,
                active_workspace: 0,
                title: String::new(),
                title_color: fg_color,
                bg_color,
                fg_color,
                clock: String::new(),
                battery: 0,
                volume: 0,
                corner_radius: 6,
                show_workspaces,
                show_clock,
                show_volume,
                show_battery,
                workspace_count,
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

    pub fn set_title_color(&self, color: u32) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).title_color = color;
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

    pub fn set_battery(&self, level: u32) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).battery = level.min(100);
                self.update();
            }
        }
    }

    pub fn set_volume(&self, level: u32) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).volume = level.min(100);
                self.update();
            }
        }
    }

    pub fn set_workspace_count(&self, count: usize) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).workspace_count = count;
                (*ptr).workspaces = (1..=count).map(|i| i.to_string()).collect();
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

pub fn get_battery_level() -> u32 {
    unsafe {
        let mut status = SYSTEM_POWER_STATUS::default();
        if GetSystemPowerStatus(&mut status).is_ok() {
            status.BatteryLifePercent as u32
        } else {
            0
        }
    }
}

pub fn get_volume_level() -> u32 {
    unsafe {
        let mut vol: u32 = 0;
        let hwo = HWAVEOUT(WAVE_MAPPER as *mut _);
        let result = waveOutGetVolume(hwo, &mut vol);
        if result == 0 {
            let left = (vol & 0xFFFF) as u32;
            let right = ((vol >> 16) & 0xFFFF) as u32;
            let avg = (left + right) / 2;
            ((avg as f32 / 0xFFFF as f32) * 100.0).round() as u32
        } else {
            0
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
            if state.show_workspaces {
                for (i, ws) in state.workspaces.iter().enumerate() {
                    if i >= state.workspace_count { break; }
                    let ws_text = format!(" {} ", ws);
                    let ws_w: Vec<u16> = ws_text.encode_utf16().chain(Some(0)).collect();

                    if i == state.active_workspace {
                        let _ = SetTextColor(hdc, COLORREF(state.bg_color));
                        RoundRect(
                            hdc,
                            x, 4, x + 36, 26,
                            state.corner_radius, state.corner_radius,
                        );
                        let active_brush = CreateSolidBrush(COLORREF(state.fg_color));
                        let _ = FillRect(hdc, &RECT { left: x + 1, top: 5, right: x + 35, bottom: 25 }, active_brush);
                        let _ = DeleteObject(active_brush);
                    } else {
                        let _ = SetTextColor(hdc, COLORREF(state.fg_color));
                        RoundRect(
                            hdc,
                            x, 4, x + 36, 26,
                            state.corner_radius, state.corner_radius,
                        );
                    }

                    let mut ws_rect = RECT {
                        left: x,
                        top: 2,
                        right: x + 40,
                        bottom: 28,
                    };
                    let _ = DrawTextW(
                        hdc,
                        &mut ws_w.clone(),
                        &mut ws_rect,
                        DT_VCENTER | DT_SINGLELINE | DT_CENTER,
                    );

                    x += 40;
                }
                x += 10;
            }

            // Draw clock (right-aligned)
            let mut right_x = 9999i32;
            if state.show_battery {
                right_x -= 80;
            }
            if state.show_volume {
                right_x -= 80;
            }

            // Draw title (with ellipsis truncation and accent color)
            if !state.title.is_empty() {
                let _ = SetTextColor(hdc, COLORREF(state.title_color));
                let title_text = format!(" {} ", state.title);
                let title_w: Vec<u16> = title_text.encode_utf16().chain(Some(0)).collect();
                let mut title_rect = RECT {
                    left: x + 10,
                    top: 0,
                    right: right_x - 10,
                    bottom: 9999,
                };
                let _ = DrawTextW(
                    hdc,
                    &mut title_w.clone(),
                    &mut title_rect,
                    DT_VCENTER | DT_SINGLELINE | DT_LEFT | DT_END_ELLIPSIS,
                );
            }
            if state.show_clock && !state.clock.is_empty() {
                let clock_w: Vec<u16> = state.clock.encode_utf16().chain(Some(0)).collect();
                let mut clock_rect = RECT {
                    left: right_x,
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

            // Draw volume indicator
            if state.show_volume {
                right_x += 80;
                let vol_text = format!(" {}% ", state.volume);
                let vol_w: Vec<u16> = vol_text.encode_utf16().chain(Some(0)).collect();
                let mut vol_rect = RECT {
                    left: right_x - 70,
                    top: 0,
                    right: right_x + 60,
                    bottom: 9999,
                };
                let _ = SetTextColor(hdc, COLORREF(state.fg_color));
                let _ = DrawTextW(
                    hdc,
                    &mut vol_w.clone(),
                    &mut vol_rect,
                    DT_VCENTER | DT_SINGLELINE | DT_RIGHT,
                );
            }

            // Draw battery indicator
            if state.show_battery {
                let bat_text = format!(" {}% ", state.battery);
                let bat_w: Vec<u16> = bat_text.encode_utf16().chain(Some(0)).collect();
                let bat_x = 9999 - 80;
                let mut bat_rect = RECT {
                    left: bat_x - 60,
                    top: 0,
                    right: bat_x + 60,
                    bottom: 9999,
                };
                let bat_color = if state.battery > 20 { state.fg_color } else { 0xFFFF4444 };
                let _ = SetTextColor(hdc, COLORREF(bat_color));
                let _ = DrawTextW(
                    hdc,
                    &mut bat_w.clone(),
                    &mut bat_rect,
                    DT_VCENTER | DT_SINGLELINE | DT_RIGHT,
                );
            }
            let _ = SetTextColor(hdc, COLORREF(state.fg_color));

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                let state = &*ptr;
                let x_pos = (lparam.0 & 0xFFFF) as i32;
                if state.show_workspaces {
                    for (i, _ws) in state.workspaces.iter().enumerate() {
                        if i >= state.workspace_count { break; }
                        let ws_left = 10i32 + (i * 40) as i32;
                        let ws_right = ws_left + 36;
                        if x_pos >= ws_left && x_pos < ws_right {
                            // Post workspace switch to platform message loop
                            let msg = WM_BAR_WORKSPACE_CLICK;
                            let _ = PostMessageW(HWND(std::ptr::null_mut()), msg, WPARAM(i), LPARAM(0));
                            break;
                        }
                    }
                }
            }
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
