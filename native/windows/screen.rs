use std::mem::size_of;
use windows::Win32::{
    Foundation::POINT,
    Graphics::Gdi::{
        DEVMODEW, ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_FLAGS, EnumDisplaySettingsExW,
        GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFOEXW, MonitorFromPoint,
    },
    UI::Shell::GetScaleFactorForMonitor,
};
use windows::core::PCWSTR;

/** 读取主屏信息 */
pub(super) fn read_screen() -> Result<(u32, u32, Option<f64>), String> {
    // SAFETY: 原点坐标有效
    let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    if monitor.is_invalid() {
        return Err("Primary monitor is unavailable".to_owned());
    }
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: 扩展结构足够
    unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }
        .ok()
        .map_err(|error| format!("GetMonitorInfoW: {error}"))?;
    let mut mode = DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    // SAFETY: 设备宽串有效
    unsafe {
        EnumDisplaySettingsExW(
            PCWSTR(info.szDevice.as_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut mode,
            ENUM_DISPLAY_SETTINGS_FLAGS(0),
        )
    }
    .ok()
    .map_err(|error| format!("EnumDisplaySettingsExW: {error}"))?;
    if mode.dmPelsWidth == 0 || mode.dmPelsHeight == 0 {
        return Err("Invalid primary display dimensions".to_owned());
    }
    // SAFETY: 显示句柄有效
    let scale = unsafe { GetScaleFactorForMonitor(monitor) }
        .map_err(|error| format!("GetScaleFactorForMonitor: {error}"))?;
    if scale.0 <= 0 {
        return Err("Invalid monitor scale factor".to_owned());
    }
    Ok((
        mode.dmPelsWidth,
        mode.dmPelsHeight,
        Some(f64::from(scale.0) / 100.0),
    ))
}
