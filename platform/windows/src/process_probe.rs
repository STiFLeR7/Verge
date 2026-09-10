use std::time::{Duration, SystemTime};
use verge_core::ports::ProcessProbe;
use windows::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, FILETIME};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

pub struct WindowsProcessProbe;

#[cfg(test)]
#[test]
fn validates_current_process_creation_time() {
    let start = WindowsProcessProbe
        .started_at(std::process::id())
        .unwrap()
        .unwrap();
    assert!(start <= SystemTime::now());
    assert!(WindowsProcessProbe.started_at(u32::MAX).unwrap().is_none());
}

impl ProcessProbe for WindowsProcessProbe {
    fn started_at(&self, pid: u32) -> std::io::Result<Option<SystemTime>> {
        unsafe {
            let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                Ok(handle) => handle,
                Err(e)
                    if e.code()
                        == windows::core::HRESULT::from_win32(ERROR_INVALID_PARAMETER.0) =>
                {
                    return Ok(None)
                }
                Err(e) => return Err(std::io::Error::other(e)),
            };
            let result = (|| {
                let mut exit = 0;
                GetExitCodeProcess(handle, &mut exit).map_err(std::io::Error::other)?;
                if exit != 259 {
                    return Ok(None);
                }
                let (mut created, mut ended, mut kernel, mut user) = (
                    FILETIME::default(),
                    FILETIME::default(),
                    FILETIME::default(),
                    FILETIME::default(),
                );
                GetProcessTimes(handle, &mut created, &mut ended, &mut kernel, &mut user)
                    .map_err(std::io::Error::other)?;
                let ticks = ((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64;
                Ok(ticks
                    .checked_sub(116_444_736_000_000_000)
                    .map(|ticks| SystemTime::UNIX_EPOCH + Duration::from_nanos(ticks * 100)))
            })();
            let _ = CloseHandle(handle);
            result
        }
    }
}
