//! Windows taskbar integration; all COM calls stay on GPUI's UI thread.
use crate::batch_progress::Snapshot;
use gpui::Window;

pub struct TaskbarProgress {
    #[cfg(windows)]
    native: Option<native::NativeProgress>,
}

impl TaskbarProgress {
    pub fn new(window: &Window) -> Self {
        #[cfg(windows)]
        {
            Self {
                native: native::NativeProgress::new(window),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = window;
            Self {}
        }
    }
    /// Refresh active progress periodically, even when the GPUI window is minimized.
    /// A shell failure is non-fatal and is retried by the next timer tick.
    pub fn update(&mut self, progress: Option<Snapshot>) -> bool {
        #[cfg(windows)]
        {
            self.native
                .as_mut()
                .is_some_and(|native| native.update(progress))
        }
        #[cfg(not(windows))]
        {
            let _ = progress;
            false
        }
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::time::{Duration, Instant};
    use windows::Win32::{
        Foundation::HWND,
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
        UI::Shell::{ITaskbarList3, TBPF_NOPROGRESS, TBPF_NORMAL, TaskbarList},
    };

    pub(super) struct NativeProgress {
        hwnd: HWND,
        interface: Option<ITaskbarList3>,
        applied: Option<Option<Snapshot>>,
        last_attempt: Option<Instant>,
    }
    impl NativeProgress {
        pub(super) fn new(window: &Window) -> Option<Self> {
            let handle = HasWindowHandle::window_handle(window).ok()?;
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                return None;
            };
            Some(Self {
                hwnd: HWND(handle.hwnd.get() as *mut std::ffi::c_void),
                interface: None,
                applied: None,
                last_attempt: None,
            })
        }
        pub(super) fn update(&mut self, progress: Option<Snapshot>) -> bool {
            let recent = self
                .last_attempt
                .is_some_and(|t| t.elapsed() < Duration::from_secs(1));
            if self.applied == Some(progress) && (progress.is_none() || recent) {
                return true;
            }
            if self.applied.is_none() && recent {
                return false;
            }
            self.last_attempt = Some(Instant::now());
            // GPUI initializes OLE/COM before creating windows on this UI thread.
            // The HWND belongs to this view; no worker thread uses this interface.
            let result = unsafe {
                (|| -> windows::core::Result<()> {
                    if self.interface.is_none() {
                        let interface: ITaskbarList3 =
                            CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)?;
                        interface.HrInit()?;
                        self.interface = Some(interface);
                    }
                    let interface = self.interface.as_ref().unwrap();
                    if let Some(progress) = progress.filter(|value| value.total > 0) {
                        interface.SetProgressState(self.hwnd, TBPF_NORMAL)?;
                        interface.SetProgressValue(
                            self.hwnd,
                            progress.completed.min(progress.total),
                            progress.total,
                        )?;
                    } else {
                        interface.SetProgressState(self.hwnd, TBPF_NOPROGRESS)?;
                    }
                    Ok(())
                })()
            };
            if result.is_ok() {
                self.applied = Some(progress);
                true
            } else {
                self.interface = None;
                self.applied = None;
                false
            }
        }
    }
    impl Drop for NativeProgress {
        fn drop(&mut self) {
            if let Some(interface) = &self.interface {
                // If the window is already gone Windows simply rejects the call.
                let _ = unsafe { interface.SetProgressState(self.hwnd, TBPF_NOPROGRESS) };
            }
        }
    }
}
