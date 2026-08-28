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

const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;
const WCA_ACCENT_POLICY: u32 = 19;

type SetWindowCompositionAttributeFn = unsafe extern "system" fn(HWND, *mut WindowCompositionAttributeData) -> i32;

pub fn enable_blur(hwnd: HWND, accent_color: u32) -> anyhow::Result<()> {
    unsafe {
        if let Some(func) = get_set_window_composition_attribute() {
            let mut accent = AccentPolicy {
                accent_state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
                accent_flags: 2,
                gradient_color: accent_color | 0xCC000000, // ABGR: alpha=0xCC
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
