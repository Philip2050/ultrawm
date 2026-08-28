use std::ptr::null_mut;
use windows::Win32::{
    Foundation::*,
    UI::{
        Input::KeyboardAndMouse::*,
        WindowsAndMessaging::*,
    },
};

pub static mut PLATFORM_PTR: *mut crate::platform::Platform = std::ptr::null_mut();

pub struct KeyboardHook {
    _hook: HHOOK,
}

impl KeyboardHook {
    pub unsafe fn install(platform: &mut crate::platform::Platform) -> anyhow::Result<Self> {
        PLATFORM_PTR = platform as *mut _;
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), HINSTANCE(null_mut()), 0)
            .map_err(|e| anyhow::anyhow!("Failed to install keyboard hook: {:?}", e))?;
        log::debug!("Keyboard hook installed");
        Ok(Self { _hook: hook })
    }
}

#[unsafe(no_mangle)]
unsafe extern "system" fn keyboard_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode < 0 {
        return CallNextHookEx(HHOOK(null_mut()), ncode, wparam, lparam);
    }

    let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
    let vk = kb.vkCode as u32;
    let pressed = wparam.0 == WM_KEYDOWN as usize || wparam.0 == WM_SYSKEYDOWN as usize;

    if !pressed {
        return CallNextHookEx(HHOOK(null_mut()), ncode, wparam, lparam);
    }

    let win = (GetKeyState(VK_LWIN.0 as i32) & 0x8000u16 as i16) != 0
        || (GetKeyState(VK_RWIN.0 as i32) & 0x8000u16 as i16) != 0;
    let ctrl = (GetKeyState(VK_CONTROL.0 as i32) & 0x8000u16 as i16) != 0;
    let shift = (GetKeyState(VK_SHIFT.0 as i32) & 0x8000u16 as i16) != 0;
    let alt = (GetKeyState(VK_MENU.0 as i32) & 0x8000u16 as i16) != 0;

    if !win && vk != VK_LWIN.0 as u32 && vk != VK_RWIN.0 as u32 {
        return CallNextHookEx(HHOOK(null_mut()), ncode, wparam, lparam);
    }

    // Swallow the Win key itself to prevent Start Menu
    if vk == VK_LWIN.0 as u32 || vk == VK_RWIN.0 as u32 {
        return LRESULT(1);
    }

    let platform = match PLATFORM_PTR.as_mut() {
        Some(p) => p,
        None => return CallNextHookEx(HHOOK(null_mut()), ncode, wparam, lparam),
    };

    match vk {
        x if x == VK_ESCAPE.0 as u32 => {
            if platform.overview {
                platform.toggle_overview();
                return LRESULT(1);
            }
        }
        x if x == VK_LEFT.0 as u32 => {
            if ctrl {
                platform.pan_camera(0, -1);
            } else if shift {
                platform.move_window(0, -1);
            } else {
                platform.move_focus(0, -1);
            }
            return LRESULT(1);
        }
        x if x == VK_RIGHT.0 as u32 => {
            if ctrl {
                platform.pan_camera(0, 1);
            } else if shift {
                platform.move_window(0, 1);
            } else {
                platform.move_focus(0, 1);
            }
            return LRESULT(1);
        }
        x if x == VK_UP.0 as u32 => {
            if ctrl {
                platform.pan_camera(-1, 0);
            } else if shift {
                platform.move_window(-1, 0);
            } else {
                platform.move_focus(-1, 0);
            }
            return LRESULT(1);
        }
        x if x == VK_DOWN.0 as u32 => {
            if ctrl {
                platform.pan_camera(1, 0);
            } else if shift {
                platform.move_window(1, 0);
            } else {
                platform.move_focus(1, 0);
            }
            return LRESULT(1);
        }
        x if x == VK_OEM_MINUS.0 as u32 => {
            if shift {
                platform.resize_height(false);
            } else {
                platform.resize_width(false);
            }
            return LRESULT(1);
        }
        x if x == VK_OEM_PLUS.0 as u32 => {
            if shift {
                platform.resize_height(true);
            } else {
                platform.resize_width(true);
            }
            return LRESULT(1);
        }
        0x46 => {
            // F — fullscreen toggle (Win+F), maximize toggle (Win+Shift+F)
            if shift {
                platform.toggle_maximize();
            } else {
                platform.toggle_fullscreen();
            }
            return LRESULT(1);
        }
        0x43 => {
            // C — close / float
            if shift {
                platform.toggle_floating();
            } else {
                platform.close_focused();
            }
            return LRESULT(1);
        }
        0x59 => {
            // Y — toggle sticky
            platform.toggle_sticky();
            return LRESULT(1);
        }
        0x54 => {
            // T — next theme (Win+T), prev theme (Win+Shift+T), or tab (Win+Alt+T)
            if alt {
                if shift {
                    platform.untab_focused();
                } else {
                    platform.tab_focused();
                }
            } else if shift {
                platform.cycle_theme(false);
            } else {
                platform.next_theme();
            }
            return LRESULT(1);
        }
        x if x >= 0x30 && x <= 0x39 => {
            // 0-9 — switch workspace (Win+0/1/2/3/4/5/6/7/8/9) or move window (Win+Shift+0..9)
            let ws_num = x - 0x30;
            let ws_count = platform.config.layout.workspace_count.max(1);
            let ws = if ws_num == 0 { ws_count - 1 } else { (ws_num - 1) as usize };
            if ws < ws_count {
                if shift {
                    platform.move_focused_window_to_workspace(ws);
                } else {
                    platform.switch_workspace(ws);
                }
            }
            return LRESULT(1);
        }
        0x47 => {
            // G — theme picker
            platform.toggle_theme_picker();
            return LRESULT(1);
        }
        0x20 => {
            // Space — launcher
            platform.toggle_launcher();
            return LRESULT(1);
        }
        0x57 => {
            // W — overview toggle
            platform.toggle_overview();
            return LRESULT(1);
        }
        0x53 => {
            // S — scratchpad toggle
            platform.toggle_scratchpad();
            return LRESULT(1);
        }
        0x48 => {
            // H — split horizontally (Win+Alt+H)
            if alt {
                platform.split_focused(true);
                return LRESULT(1);
            }
        }
        0x56 => {
            // V — split vertically (Win+Alt+V)
            if alt {
                platform.split_focused(false);
                return LRESULT(1);
            }
        }
        0x55 => {
            // U — unsplit (Win+Alt+U)
            if alt {
                platform.unsplit_focused();
                return LRESULT(1);
            }
        }
        0x4D => {
            // M — minimize (Win+M) / restore (Win+Shift+M)
            if shift {
                platform.restore_minimized();
            } else {
                platform.minimize_focused();
            }
            return LRESULT(1);
        }
        0x4F => {
            // O — toggle always-on-top
            platform.toggle_always_on_top();
            return LRESULT(1);
        }
        x if x == VK_OEM_COMMA.0 as u32 => {
            // Comma — shrink gaps (Win+,)
            platform.adjust_gap(-1);
            return LRESULT(1);
        }
        x if x == VK_OEM_PERIOD.0 as u32 => {
            // Period — grow gaps (Win+.)
            platform.adjust_gap(1);
            return LRESULT(1);
        }
        _ => {}
    }

    CallNextHookEx(HHOOK(null_mut()), ncode, wparam, lparam)
}
