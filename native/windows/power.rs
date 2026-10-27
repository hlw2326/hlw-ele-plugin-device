use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

/** 读取电池状态 */
pub(super) fn read_battery() -> Result<Option<bool>, String> {
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: 独占结构可写
    unsafe { GetSystemPowerStatus(&mut status) }.map_err(|error| error.to_string())?;
    Ok(decode_battery(status.BatteryFlag))
}

fn decode_battery(flag: u8) -> Option<bool> {
    (flag != 255).then_some(flag & 128 == 0)
}

#[cfg(test)]
mod tests {
    use super::decode_battery;

    #[test]
    fn distinguishes_absence_and_unknown() {
        assert_eq!(decode_battery(255), None);
        assert_eq!(decode_battery(128), Some(false));
        assert_eq!(decode_battery(8), Some(true));
        assert_eq!(decode_battery(1), Some(true));
    }
}
