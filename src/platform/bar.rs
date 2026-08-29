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
    cpu: u32,
    memory: u32,
    corner_radius: i32,
    show_workspaces: bool,
    show_clock: bool,
    show_volume: bool,
    show_battery: bool,
    show_cpu: bool,
    show_memory: bool,
    workspace_count: usize,
    window_counts: Vec<usize>, // window count per workspace
    snap_mode: bool,
    monocle_mode: bool,
    reload_flash: u32, // frames remaining for green reload flash
    resize_flash: u32, // frames remaining for resize size indicator
    resize_size: String, // current window size shown during resize
    network: bool,
    title_scroll_offset: i32, // current scroll offset for long titles
    title_scroll_timer: u32,  // frames before scrolling starts
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
        show_cpu: bool,
        show_memory: bool,
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
                cpu: 0,
                memory: 0,
                corner_radius: 6,
                show_workspaces,
                show_clock,
                show_volume,
                show_battery,
                show_cpu,
                show_memory,
                workspace_count,
                window_counts: vec![0; workspace_count],
                snap_mode: false,
                monocle_mode: false,
                reload_flash: 0,
                resize_flash: 0,
                resize_size: String::new(),
                network: true,
                title_scroll_offset: 0,
                title_scroll_timer: 0,
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

    pub fn set_workspaces(&self, workspaces: Vec<String>, active: usize, window_counts: Vec<usize>) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).workspaces = workspaces;
                (*ptr).active_workspace = active;
                (*ptr).window_counts = window_counts;
                self.update();
            }
        }
    }

    pub fn set_title(&self, title: &str) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).title = title.to_string();
                (*ptr).title_scroll_offset = 0;
                (*ptr).title_scroll_timer = 0;
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

    pub fn set_cpu(&self, usage: u32) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).cpu = usage.min(100);
                self.update();
            }
        }
    }

    pub fn set_memory(&self, usage: u32) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).memory = usage.min(100);
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

    pub fn set_snap_mode(&self, enabled: bool) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).snap_mode = enabled;
                self.update();
            }
        }
    }

    pub fn set_monocle_mode(&self, enabled: bool) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).monocle_mode = enabled;
                self.update();
            }
        }
    }

    pub fn set_network(&self, online: bool) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).network = online;
                self.update();
            }
        }
    }

    pub fn trigger_reload_flash(&self) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).reload_flash = 30; // ~0.5 second flash at 60fps
                self.update();
            }
        }
    }

    pub fn show_resize_size(&self, size_text: String) {
        unsafe {
            let ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut BarState;
            if !ptr.is_null() {
                (*ptr).resize_flash = 60; // ~1 second display at 60fps
                (*ptr).resize_size = size_text;
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

pub fn is_network_online() -> bool {
    unsafe {
        let h = kernel32::LoadLibraryW(HSTRING::from("wininet.dll"));
        if h.is_null() { return false; }
        let func: unsafe extern "system" fn(*mut u32) -> u32 =
            std::mem::transmute(kernel32::GetProcAddress(h, s!("InternetGetConnectedState")));
        if func.is_null() {
            let _ = kernel32::FreeLibrary(h);
            return false;
        }
        let mut flags: u32 = 0;
        let result = (func.unwrap())(&mut flags);
        let _ = kernel32::FreeLibrary(h);
        result != 0
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
            let state = &mut *ptr;

            // Decay reload flash
            if state.reload_flash > 0 {
                state.reload_flash -= 1;
            }

            // Decay resize size flash
            if state.resize_flash > 0 {
                state.resize_flash -= 1;
            }

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

            // Draw workspace indicators with window counts
            let mut x = 10i32;
            if state.show_workspaces {
                for (i, ws) in state.workspaces.iter().enumerate() {
                    if i >= state.workspace_count { break; }
                    let count = state.window_counts.get(i).copied().unwrap_or(0);
                    let ws_text = if count > 0 {
                        format!(" {} ({}) ", ws, count)
                    } else {
                        format!(" {} ", ws)
                    };
                    let ws_w: Vec<u16> = ws_text.encode_utf16().chain(Some(0)).collect();

                    if i == state.active_workspace {
                        let _ = SetTextColor(hdc, COLORREF(state.bg_color));
                        RoundRect(
                            hdc,
                            x, 4, x + 44, 26,
                            state.corner_radius, state.corner_radius,
                        );
                        let active_brush = CreateSolidBrush(COLORREF(state.fg_color));
                        let _ = FillRect(hdc, &RECT { left: x + 1, top: 5, right: x + 43, bottom: 25 }, active_brush);
                        let _ = DeleteObject(active_brush);
                    } else {
                        let _ = SetTextColor(hdc, COLORREF(state.fg_color));
                        RoundRect(
                            hdc,
                            x, 4, x + 44, 26,
                            state.corner_radius, state.corner_radius,
                        );
                    }

                    let mut ws_rect = RECT {
                        left: x,
                        top: 2,
                        right: x + 44,
                        bottom: 28,
                    };
                    let _ = DrawTextW(
                        hdc,
                        &mut ws_w.clone(),
                        &mut ws_rect,
                        DT_VCENTER | DT_SINGLELINE | DT_CENTER,
                    );

                    x += 48;
                }
                x += 10;
            }

            // Draw snap mode indicator
            if state.snap_mode {
                let snap_text = " SNAP ";
                let snap_w: Vec<u16> = snap_text.encode_utf16().chain(Some(0)).collect();
                let snap_color = 0xFF00FFFF; // Cyan
                let _ = SetTextColor(hdc, COLORREF(snap_color));
                let mut snap_rect = RECT {
                    left: x + 5,
                    top: 4,
                    right: x + 65,
                    bottom: 26,
                };
                let _ = DrawTextW(
                    hdc,
                    &mut snap_w.clone(),
                    &mut snap_rect,
                    DT_VCENTER | DT_SINGLELINE | DT_LEFT,
                );
                x += 60;
            }

            // Draw monocle mode indicator
            if state.monocle_mode {
                let mono_text = " MONOCLE ";
                let mono_w: Vec<u16> = mono_text.encode_utf16().chain(Some(0)).collect();
                let mono_color = 0xFFFF8800; // Orange
                let _ = SetTextColor(hdc, COLORREF(mono_color));
                let mut mono_rect = RECT {
                    left: x + 5,
                    top: 4,
                    right: x + 80,
                    bottom: 26,
                };
                let _ = DrawTextW(
                    hdc,
                    &mut mono_w.clone(),
                    &mut mono_rect,
                    DT_VCENTER | DT_SINGLELINE | DT_LEFT,
                );
                x += 76;
            }

            // Draw network indicator
            let net_color = if state.network { 0xFFA6E3A1 } else { 0xFFF38BA8 }; // green/red
            let net_text = if state.network { " W " } else { " W! " };
            let net_w: Vec<u16> = net_text.encode_utf16().chain(Some(0)).collect();
            let _ = SetTextColor(hdc, COLORREF(net_color));
            let mut net_rect = RECT {
                left: x + 5,
                top: 4,
                right: x + 45,
                bottom: 26,
            };
            let _ = DrawTextW(
                hdc,
                &mut net_w.clone(),
                &mut net_rect,
                DT_VCENTER | DT_SINGLELINE | DT_LEFT,
            );
            x += 40;

            // Draw clock (right-aligned)
            let mut right_x = 9999i32;
            if state.show_battery {
                right_x -= 80;
            }
            if state.show_volume {
                right_x -= 80;
            }

            // Draw title (with scrolling for long titles)
            if !state.title.is_empty() {
                let _ = SetTextColor(hdc, COLORREF(state.title_color));
                let title_text = format!(" {} ", state.title);
                let title_w: Vec<u16> = title_text.encode_utf16().chain(Some(0)).collect();
                let avail_width = (right_x - 10) - (x + 10);
                if avail_width > 20 {
                    let mut text_size = SIZE::default();
                    let _ = GetTextExtentPoint32W(hdc, &title_w, &mut text_size);
                    let text_w = text_size.cx;
                    let draw_x = if text_w > avail_width {
                        // Scrolling title
                        state.title_scroll_timer = state.title_scroll_timer.saturating_sub(1);
                        if state.title_scroll_timer == 0 {
                            state.title_scroll_offset -= 1;
                            if state.title_scroll_offset < -(text_w + 20) {
                                state.title_scroll_offset = avail_width + 10;
                                state.title_scroll_timer = 60;
                            }
                        }
                        x + 10 + state.title_scroll_offset
                    } else {
                        state.title_scroll_offset = 0;
                        state.title_scroll_timer = 0;
                        x + 10
                    };
                    let _ = TextOutW(hdc, draw_x, 8, &title_w);
                }
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

            // Draw CPU indicator
            if state.show_cpu {
                let cpu_text = format!(" CPU:{}% ", state.cpu);
                let cpu_w: Vec<u16> = cpu_text.encode_utf16().chain(Some(0)).collect();
                let cpu_x = if state.show_battery { 9999 - 160 } else { 9999 - 80 };
                let mut cpu_rect = RECT {
                    left: cpu_x - 70,
                    top: 0,
                    right: cpu_x + 60,
                    bottom: 9999,
                };
                let _ = SetTextColor(hdc, COLORREF(state.fg_color));
                let _ = DrawTextW(
                    hdc,
                    &mut cpu_w.clone(),
                    &mut cpu_rect,
                    DT_VCENTER | DT_SINGLELINE | DT_RIGHT,
                );
            }

            // Draw memory indicator
            if state.show_memory {
                let mem_text = format!(" MEM:{}% ", state.memory);
                let mem_w: Vec<u16> = mem_text.encode_utf16().chain(Some(0)).collect();
                let mem_x = if state.show_battery && state.show_cpu {
                    9999 - 250
                } else if state.show_battery || state.show_cpu {
                    9999 - 160
                } else {
                    9999 - 80
                };
                let mut mem_rect = RECT {
                    left: mem_x - 70,
                    top: 0,
                    right: mem_x + 60,
                    bottom: 9999,
                };
                let _ = SetTextColor(hdc, COLORREF(state.fg_color));
                let _ = DrawTextW(
                    hdc,
                    &mut mem_w.clone(),
                    &mut mem_rect,
                    DT_VCENTER | DT_SINGLELINE | DT_RIGHT,
                );
            }
            let _ = SetTextColor(hdc, COLORREF(state.fg_color));

            // Draw reload flash overlay (green tint)
            if state.reload_flash > 0 {
                let alpha = state.reload_flash.min(30) as f32 / 30.0;
                let flash_green = (alpha * 80.0) as u8;
                let flash_color = (flash_green as u32) | ((flash_green as u32) << 8);
                let flash_brush = CreateSolidBrush(COLORREF(flash_color));
                let _ = FillRect(hdc, &RECT { left: 0, top: 0, right: 9999, bottom: 9999 }, flash_brush);
                let _ = DeleteObject(flash_brush);
            }

            // Draw resize size indicator
            if state.resize_flash > 0 {
                let alpha = state.resize_flash.min(60) as f32 / 60.0;
                let box_w = 140;
                let box_h = 28;
                let cx = 9999 / 2;
                let cy = 20; // top area of bar
                let bx = cx - box_w / 2;
                let by = cy - box_h / 2;

                // Background box
                let bg_brush = CreateSolidBrush(COLORREF(state.bg_color));
                let _ = FillRect(hdc, &RECT { left: bx, top: by, right: bx + box_w, bottom: by + box_h }, bg_brush);
                let _ = DeleteObject(bg_brush);

                // Border
                let border_pen = CreatePen(PS_SOLID, 1, COLORREF(state.fg_color));
                let old_pen = SelectObject(hdc, border_pen);
                let _ = Rectangle(hdc, bx, by, bx + box_w, by + box_h);
                let _ = SelectObject(hdc, old_pen);
                let _ = DeleteObject(border_pen);

                // Text
                let _ = SetTextColor(hdc, COLORREF(state.fg_color));
                let size_w: Vec<u16> = state.resize_size.encode_utf16().chain(Some(0)).collect();
                let mut size_rect = RECT {
                    left: bx,
                    top: by,
                    right: bx + box_w,
                    bottom: by + box_h,
                };
                let _ = DrawTextW(
                    hdc,
                    &mut size_w.clone(),
                    &mut size_rect,
                    DT_VCENTER | DT_SINGLELINE | DT_CENTER,
                );
            }

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
                        let ws_left = 10i32 + (i * 48) as i32;
                        let ws_right = ws_left + 44;
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
