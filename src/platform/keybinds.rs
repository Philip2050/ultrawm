use windows::Win32::UI::Input::KeyboardAndMouse::*;

#[derive(Debug, Clone, Copy)]
pub struct ParsedKeybinds {
    pub focus_left: u32,
    pub focus_right: u32,
    pub focus_up: u32,
    pub focus_down: u32,
    pub move_left: u32,
    pub move_right: u32,
    pub move_up: u32,
    pub move_down: u32,
    pub pan_left: u32,
    pub pan_right: u32,
    pub pan_up: u32,
    pub pan_down: u32,
    pub grow_width: u32,
    pub shrink_width: u32,
    pub grow_height: u32,
    pub shrink_height: u32,
    pub fullscreen: u32,
    pub close: u32,
    pub float: u32,
    pub sticky: u32,
    pub theme_next: u32,
    pub theme_prev: u32,
    pub theme_picker: u32,
    pub launcher: u32,
}

pub fn parse_keybind(s: &str) -> u32 {
    match s.to_lowercase().as_str() {
        "left" => VK_LEFT.0 as u32,
        "right" => VK_RIGHT.0 as u32,
        "up" => VK_UP.0 as u32,
        "down" => VK_DOWN.0 as u32,
        "space" => VK_SPACE.0 as u32,
        "escape" => VK_ESCAPE.0 as u32,
        "oemminus" => VK_OEM_MINUS.0 as u32,
        "oemplus" => VK_OEM_PLUS.0 as u32,
        "oemcomma" => VK_OEM_COMMA.0 as u32,
        "oemperiod" => VK_OEM_PERIOD.0 as u32,
        "oem1" => VK_OEM_1.0 as u32,
        "oem2" => VK_OEM_2.0 as u32,
        "oem3" => VK_OEM_3.0 as u32,
        "oem4" => VK_OEM_4.0 as u32,
        "oem5" => VK_OEM_5.0 as u32,
        "oem6" => VK_OEM_6.0 as u32,
        "oem7" => VK_OEM_7.0 as u32,
        "oem8" => VK_OEM_8.0 as u32,
        "oem102" => VK_OEM_102.0 as u32,
        "tab" => VK_TAB.0 as u32,
        "return" | "enter" => VK_RETURN.0 as u32,
        "back" => VK_BACK.0 as u32,
        "lwin" => VK_LWIN.0 as u32,
        "rwin" => VK_RWIN.0 as u32,
        "apps" => VK_APPS.0 as u32,
        "lshift" => VK_LSHIFT.0 as u32,
        "rshift" => VK_RSHIFT.0 as u32,
        "lcontrol" => VK_LCONTROL.0 as u32,
        "rcontrol" => VK_RCONTROL.0 as u32,
        "lmenu" => VK_LMENU.0 as u32,
        "rmenu" => VK_RMENU.0 as u32,
        "capital" => VK_CAPITAL.0 as u32,
        "escape" => VK_ESCAPE.0 as u32,
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase() as u32
            } else {
                VK_ESCAPE.0 as u32
            }
        }
        _ => 0,
    }
}

pub fn parse_keybinds(config: &crate::config::KeybindsConfig) -> ParsedKeybinds {
    ParsedKeybinds {
        focus_left: parse_keybind(&config.focus_left),
        focus_right: parse_keybind(&config.focus_right),
        focus_up: parse_keybind(&config.focus_up),
        focus_down: parse_keybind(&config.focus_down),
        move_left: parse_keybind(&config.move_left),
        move_right: parse_keybind(&config.move_right),
        move_up: parse_keybind(&config.move_up),
        move_down: parse_keybind(&config.move_down),
        pan_left: parse_keybind(&config.pan_left),
        pan_right: parse_keybind(&config.pan_right),
        pan_up: parse_keybind(&config.pan_up),
        pan_down: parse_keybind(&config.pan_down),
        grow_width: parse_keybind(&config.grow_width),
        shrink_width: parse_keybind(&config.shrink_width),
        grow_height: parse_keybind(&config.grow_height),
        shrink_height: parse_keybind(&config.shrink_height),
        fullscreen: parse_keybind(&config.fullscreen),
        close: parse_keybind(&config.close),
        float: parse_keybind(&config.float),
        sticky: parse_keybind(&config.sticky),
        theme_next: parse_keybind(&config.theme_next),
        theme_prev: parse_keybind(&config.theme_prev),
        theme_picker: parse_keybind(&config.theme_picker),
        launcher: parse_keybind(&config.launcher),
    }
}
