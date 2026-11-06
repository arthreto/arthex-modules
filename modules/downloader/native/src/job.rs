//! Runs yt-dlp and whatever it starts in a group that dies with this program, so that cancelling
//! leaves nothing running: it starts `FFmpeg` by itself.

use std::process::Child;

#[cfg(windows)]
mod system {
    use std::{os::windows::io::AsRawHandle, process::Child, ptr};

    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject,
        },
    };

    pub struct Group(HANDLE);

    // SAFETY: a job handle can be used from any thread.
    unsafe impl Send for Group {}
    // SAFETY: as above, and the calls here change nothing on this side.
    unsafe impl Sync for Group {}

    impl Group {
        pub fn holding(child: &Child) -> Option<Self> {
            // SAFETY: a new unnamed job, closed in `drop`.
            let job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
            if job.is_null() {
                return None;
            }
            let group = Self(job);
            // SAFETY: every other field of this C struct is valid when zero.
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let size = u32::try_from(size_of_val(&limits)).ok()?;
            // SAFETY: the structure outlives the call, and the handles are valid.
            let done = unsafe {
                SetInformationJobObject(
                    group.0,
                    JobObjectExtendedLimitInformation,
                    (&raw const limits).cast(),
                    size,
                ) != 0
                    && AssignProcessToJobObject(group.0, child.as_raw_handle()) != 0
            };
            done.then_some(group)
        }

        pub fn kill(&self) {
            // SAFETY: our own job; failing only leaves it to end by itself.
            unsafe { TerminateJobObject(self.0, 1) };
        }
    }

    impl Drop for Group {
        fn drop(&mut self) {
            // SAFETY: created in `holding` and used nowhere else.
            unsafe { CloseHandle(self.0) };
        }
    }
}

#[cfg(not(windows))]
mod system {
    use std::process::Child;

    /// The program is only compiled for the checks outside Windows.
    pub struct Group;

    impl Group {
        pub fn holding(_: &Child) -> Option<Self> {
            None
        }

        #[allow(clippy::unused_self)]
        pub fn kill(&self) {}
    }
}

pub use system::Group;

/// Stops it, with what it started when a group holds them.
pub fn stop(group: Option<&Group>, child: &mut Child) {
    if let Some(group) = group {
        group.kill();
    }
    let _ = child.kill();
}
