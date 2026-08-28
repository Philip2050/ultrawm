use std::ptr;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::*,
        UI::WindowsAndMessaging::*,
    },
};

pub static mut GESTURE_PTR: *mut GestureReceiver = ptr::null_mut();

pub struct GestureReceiver {
    hwnd: HWND,
}

impl GestureReceiver {
    pub fn create(width: i32, height: i32) -> anyhow::Result<()> {
        unsafe {
            let hinstance = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(gesture_wnd_proc),
                hInstance: hinstance,
                lpszClassName: w!("UltraWMGesture"),
                hbrBackground: HBRUSH(GetStockObject(HOLLOW_BRUSH).0),
                ..Default::default()
            };
            RegisterClassW(&class);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0),
                w!("UltraWMGesture"),
                w!("UltraWM Gestures"),
                WINDOW_STYLE(WS_POPUP.0),
                0, 0, width, height,
                None, None, hinstance, None,
            )?;

            let receiver = Self { hwnd };
            let boxed = Box::new(receiver);
            let leaked = Box::leak(boxed);
            GESTURE_PTR = leaked as *mut GestureReceiver;

            Ok(())
        }
    }

    pub fn dismiss(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

impl Drop for GestureReceiver {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn gesture_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_GESTURE => {
            // Placeholder for touchpad gesture handling
            // Windows sends WM_GESTURE with GESTUREINFO for pan/zoom gestures
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
