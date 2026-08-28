use std::ptr;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::LibraryLoader::*,
        UI::WindowsAndMessaging::*,
    },
};

#[repr(C)]
struct AccentPolicy {
    accent_state: u32,
    accent_flags: u32,
    gradient_color: u32,
    animation_id: u32,
}

#[repr(C)]
struct WindowCompositionAttributeData {
    attribute: u32,
    data: *mut AccentPolicy,
    size: usize,
}

const ACCENT_ENABLE_BLURBEHIND: u32 = 3;
const WCA_ACCENT_POLICY: u32 = 19;

type SetWindowCompositionAttributeFn = unsafe extern "system" fn(HWND, *mut WindowCompositionAttributeData) -> i32;

pub fn enable_blur(hwnd: HWND) -> anyhow::Result<()> {
    unsafe {
        if let Some(func) = get_set_window_composition_attribute() {
            let mut accent = AccentPolicy {
                accent_state: ACCENT_ENABLE_BLURBEHIND,
                accent_flags: 0,
                gradient_color: 0,
                animation_id: 0,
            };

            let mut data = WindowCompositionAttributeData {
                attribute: WCA_ACCENT_POLICY,
                data: &mut accent,
                size: std::mem::size_of::<AccentPolicy>(),
            };

            let _ = func(hwnd, &mut data);
        }
    }
    Ok(())
}

fn get_set_window_composition_attribute() -> Option<SetWindowCompositionAttributeFn> {
    unsafe {
        let hmodule = GetModuleHandleW(PCWSTR(ptr::null_mut())).ok()?;
        let name = b"SetWindowCompositionAttribute\0";
        let addr = GetProcAddress(hmodule, PCSTR(name.as_ptr()));
        addr.map(|addr| std::mem::transmute(addr))
    }
}
