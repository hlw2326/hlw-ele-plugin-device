use windows::{
    Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY, RegGetValueW,
    },
    core::PCWSTR,
};

/** 读取注册字符 */
pub(super) fn registry(path: PCWSTR, name: PCWSTR) -> Result<String, String> {
    let mut size = 0;
    let flags = RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY;
    // SAFETY: 静态宽串有效
    // 空值仅查长度
    unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            path,
            name,
            flags,
            None,
            None,
            Some(&mut size),
        )
    }
    .ok()
    .map_err(|error| error.to_string())?;
    loop {
        let mut buffer = vec![0u16; (size as usize).div_ceil(2)];
        // SAFETY: 宽串持续有效
        // 缓冲容量足够
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                path,
                name,
                flags,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if status == windows::Win32::Foundation::ERROR_MORE_DATA {
            continue;
        }
        status.ok().map_err(|error| error.to_string())?;
        buffer.truncate(size as usize / 2);
        while buffer.last() == Some(&0) {
            buffer.pop();
        }
        let value = String::from_utf16(&buffer).map_err(|error| error.to_string())?;
        return if value.trim().is_empty() {
            Err("Registry string is empty".to_owned())
        } else {
            Ok(value.trim().to_owned())
        };
    }
}
