use super::registry::registry;
use windows::{
    Win32::System::Threading::{ALL_PROCESSOR_GROUPS, GetActiveProcessorCount},
    core::w,
};

/** 读取处理器 */
pub fn read_cpu() -> Result<(String, Option<u32>), String> {
    let name = registry(
        w!("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"),
        w!("ProcessorNameString"),
    )?;
    // SAFETY: 全组编号有效
    let cores = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
    if cores == 0 {
        return Err(format!(
            "GetActiveProcessorCount: {}",
            windows::core::Error::from_thread()
        ));
    }
    Ok((name, Some(cores)))
}
