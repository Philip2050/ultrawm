use std::ptr;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::*,
        UI::{
            Input::Touch::*,
            WindowsAndMessaging::*,
        },
    },
};

pub static mut GESTURE_PTR: *mut GestureReceiver = ptr::null_mut();

// Gesture IDs (raw u32 values)
const GID_PAN: u32 = 4;
const GID_ZOOM: u32 = 3;
const GID_TWOFINGERTAP: u32 = 6;

// Gesture config flags
const GC_PAN: u32 = 0x00000001;
const GC_PAN_WITH_GUTTER: u32 = 0x00000002;
const GC_PAN_WITH_INERTIA: u32 = 0x00000004;
const GC_PAN_WITH_SINGLE_FINGER: u32 = 0x00000008;
const GC_ZOOM: u32 = 0x00000010;
const GC_TWOFINGERTAP: u32 = 0x00000040;

// Gesture flags (GESTUREINFO.dwFlags)
const GF_BEGIN: u32 = 0x00000001;
const GF_INERTIA: u32 = 0x00000002;
const GF_END: u32 = 0x00000004;

const GESTURE_PAN_FACTOR: f64 = 30.0;
const GESTURE_ZOOM_FACTOR: f32 = 40.0;
const GESTURE_SWIPE_THRESHOLD: i32 = 80;

pub struct GestureReceiver {
    hwnd: HWND,
}

impl GestureReceiver {
    pub fn create(width: i32, height: i32) -> anyhow::Result<Self> {
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
                WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0 | WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0),
                w!("UltraWMGesture"),
                w!("UltraWM Gestures"),
                WINDOW_STYLE(WS_POPUP.0),
                0, 0, width, height,
                None, None, hinstance, None,
            )?;

            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA);

            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );

            let gc_config = GESTURECONFIG {
                dwID: GESTURECONFIG_ID(GID_PAN),
                dwWant: GC_PAN_WITH_SINGLE_FINGER | GC_PAN_WITH_INERTIA | GC_PAN_WITH_GUTTER,
                dwBlock: 0,
            };
            let _ = SetGestureConfig(hwnd, 0, &[gc_config], std::mem::size_of::<GESTURECONFIG>() as u32);

            let gz_config = GESTURECONFIG {
                dwID: GESTURECONFIG_ID(GID_ZOOM),
                dwWant: GC_ZOOM,
                dwBlock: 0,
            };
            let _ = SetGestureConfig(hwnd, 1, &[gz_config], std::mem::size_of::<GESTURECONFIG>() as u32);

            let gt_config = GESTURECONFIG {
                dwID: GESTURECONFIG_ID(GID_TWOFINGERTAP),
                dwWant: GC_TWOFINGERTAP,
                dwBlock: 0,
            };
            let _ = SetGestureConfig(hwnd, 2, &[gt_config], std::mem::size_of::<GESTURECONFIG>() as u32);

            let receiver = Self { hwnd };
            let boxed = Box::new(receiver);
            let leaked = Box::leak(boxed);
            GESTURE_PTR = leaked as *mut GestureReceiver;

            Ok(Self { hwnd })
        }
    }

    pub fn dismiss(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }

    pub fn resize(&self, width: i32, height: i32) {
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                0, 0, width, height,
                SWP_NOMOVE | SWP_NOACTIVATE,
            );
        }
    }
}

impl Clone for GestureReceiver {
    fn clone(&self) -> Self {
        Self { hwnd: self.hwnd }
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
            let mut gi = GESTUREINFO::default();
            let hgi = HGESTUREINFO(lparam.0 as *mut core::ffi::c_void);

            if GetGestureInfo(hgi, &mut gi).is_ok() {
                let platform = match GESTURE_PTR.as_mut() {
                    Some(p) => p,
                    None => return DefWindowProcW(hwnd, msg, wparam, lparam),
                };

                match gi.dwID {
                    id if id == GID_PAN => {
                        let _ = platform.handle_gesture_pan(&gi);
                    }
                    id if id == GID_ZOOM => {
                        let _ = platform.handle_gesture_zoom(&gi);
                    }
                    id if id == GID_TWOFINGERTAP => {
                        let _ = platform.handle_gesture_twofinger(&gi);
                    }
                    _ => {}
                }
            }

            let _ = CloseGestureInfoHandle(hgi);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

impl GestureReceiver {
    fn handle_gesture_pan(&mut self, gi: &GESTUREINFO) -> anyhow::Result<()> {
        unsafe {
            let platform = crate::platform::keyboard::PLATFORM_PTR;
            if platform.is_null() {
                return Ok(());
            }

            let platform = match platform.as_mut() {
                Some(p) => p,
                None => return Ok(()),
            };

            let flags = gi.dwFlags;
            let x = gi.ptsLocation.x as i32;
            let y = gi.ptsLocation.y as i32;

            if (flags & GF_BEGIN) != 0 {
                platform.gesture_pan_start = Some((x, y));
                platform.gesture_pan_last = Some((x, y));
            } else if (flags & GF_INERTIA) != 0 {
                let Some(last) = platform.gesture_pan_last else { return Ok(()) };
                let dx = x - last.0;
                let dy = y - last.1;

                if dx.abs() > GESTURE_SWIPE_THRESHOLD {
                    if dx < 0 {
                        platform.pan_camera(0, 1);
                    } else {
                        platform.pan_camera(0, -1);
                    }
                } else if dy.abs() > GESTURE_SWIPE_THRESHOLD {
                    if dy < 0 {
                        platform.pan_camera(1, 0);
                    } else {
                        platform.pan_camera(-1, 0);
                    }
                }

                platform.gesture_pan_start = None;
                platform.gesture_pan_last = None;
            } else if (flags & GF_END) != 0 {
                platform.gesture_pan_start = None;
                platform.gesture_pan_last = None;
            } else {
                if let Some(start) = platform.gesture_pan_start {
                    if let Some(last) = platform.gesture_pan_last {
                        let dx = x - last.0;
                        let dy = y - last.1;

                        let dist = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt();
                        if dist > GESTURE_SWIPE_THRESHOLD as f64 {
                            if dx.abs() >= dy.abs() {
                                if dx > 0 {
                                    platform.move_focus(0, 1);
                                } else {
                                    platform.move_focus(0, -1);
                                }
                            } else {
                                if dy > 0 {
                                    platform.move_focus(1, 0);
                                } else {
                                    platform.move_focus(-1, 0);
                                }
                            }
                            platform.gesture_pan_start = None;
                        }
                    }
                    platform.gesture_pan_last = Some((x, y));
                }
            }
        }

        Ok(())
    }

    fn handle_gesture_zoom(&mut self, gi: &GESTUREINFO) -> anyhow::Result<()> {
        unsafe {
            let platform = crate::platform::keyboard::PLATFORM_PTR;
            if platform.is_null() {
                return Ok(());
            }

            let platform = match platform.as_mut() {
                Some(p) => p,
                None => return Ok(()),
            };

            let zoom = (gi.ullArguments as f32 / 65536.0 - 1.0) * GESTURE_ZOOM_FACTOR;

            if zoom > 0.0 {
                platform.resize_width(true);
            } else if zoom < 0.0 {
                platform.resize_width(false);
            }
        }

        Ok(())
    }

    fn handle_gesture_twofinger(&mut self, _gi: &GESTUREINFO) -> anyhow::Result<()> {
        unsafe {
            let platform = crate::platform::keyboard::PLATFORM_PTR;
            if platform.is_null() {
                return Ok(());
            }

            let platform = match platform.as_mut() {
                Some(p) => p,
                None => return Ok(()),
            };
            platform.toggle_fullscreen();
        }

        Ok(())
    }
}
