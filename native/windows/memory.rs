use std::mem::size_of;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

/** 读取内存状态 */
pub(super) fn read_memory() -> Result<(f64, Option<f64>), String> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: 结构尺寸已填
    unsafe { GlobalMemoryStatusEx(&mut status) }.map_err(|error| error.to_string())?;
    Ok((status.ullTotalPhys as f64, Some(status.ullAvailPhys as f64)))
}
