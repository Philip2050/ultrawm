use std::collections::HashMap;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        UI::WindowsAndMessaging::*,
    },
};

pub static mut SCRATCHPAD_PTR: *mut ScratchpadManager = std::ptr::null_mut();

pub struct ScratchpadManager {
    windows: HashMap<usize, ScratchpadWindow>,
}

pub struct ScratchpadWindow {
    pub hwnd: HWND,
    pub name: String,
    pub visible: bool,
}

impl ScratchpadManager {
    pub fn create() -> anyhow::Result<()> {
        let manager = Self {
            windows: HashMap::new(),
        };

        unsafe {
            let boxed = Box::new(manager);
            let leaked = Box::leak(boxed);
            SCRATCHPAD_PTR = leaked as *mut ScratchpadManager;
        }

        Ok(())
    }

    pub fn toggle(&mut self) {
        // Toggle visibility of all scratchpad windows
        for (_, win) in self.windows.iter_mut() {
            win.visible = !win.visible;
            unsafe {
                let _ = ShowWindow(win.hwnd, if win.visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }

    pub fn add(&mut self, hwnd: HWND, name: String) {
        let key = hwnd.0 as usize;
        self.windows.insert(key, ScratchpadWindow {
            hwnd,
            name,
            visible: true,
        });
    }

    pub fn remove(&mut self, hwnd: HWND) {
        let key = hwnd.0 as usize;
        self.windows.remove(&key);
    }
}
