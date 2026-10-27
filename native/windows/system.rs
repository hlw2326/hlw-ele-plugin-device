use super::registry::registry;
use sha2::{Digest, Sha256};
use std::mem::size_of;
use windows::Wdk::System::SystemServices::RtlGetVersion;
use windows::{
    Win32::System::SystemInformation::{
        ComputerNameDnsHostname, GetComputerNameExW, GetNativeSystemInfo, OSVERSIONINFOEXW,
        OSVERSIONINFOW, PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM64,
        PROCESSOR_ARCHITECTURE_INTEL, SYSTEM_INFO,
    },
    core::{PWSTR, w},
};

pub(super) struct System {
    pub name: String,
    pub release: String,
    pub hostname: String,
    pub arch: String,
}

/** 读取系统信息 */
pub fn read_system() -> Result<System, String> {
    let path = w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
    let mut version = OSVERSIONINFOEXW {
        dwOSVersionInfoSize: size_of::<OSVERSIONINFOEXW>() as u32,
        ..Default::default()
    };
    // SAFETY: 扩展结构足够
    unsafe { RtlGetVersion((&mut version as *mut OSVERSIONINFOEXW).cast::<OSVERSIONINFOW>()) }
        .ok()
        .map_err(|error| format!("RtlGetVersion: {error}"))?;
    let name = product_name(
        registry(path, w!("ProductName"))?,
        version.dwBuildNumber,
        version.wProductType == 1,
    );
    Ok(System {
        name,
        release: format!(
            "{}.{}.{}",
            version.dwMajorVersion, version.dwMinorVersion, version.dwBuildNumber
        ),
        hostname: hostname()?,
        arch: architecture()?,
    })
}

/** 读取安装标识 */
pub fn read_identity() -> Result<String, String> {
    let value = registry(w!("SOFTWARE\\Microsoft\\Cryptography"), w!("MachineGuid"))?;
    Ok(format!("{:x}", Sha256::digest(value.as_bytes())))
}

fn product_name(name: String, build: u32, is_workstation: bool) -> String {
    if is_workstation && build >= 22000 {
        name.replacen("Windows 10", "Windows 11", 1)
    } else {
        name
    }
}

fn hostname() -> Result<String, String> {
    let mut size = 0;
    // SAFETY: 空值仅查长度
    if let Err(error) = unsafe { GetComputerNameExW(ComputerNameDnsHostname, None, &mut size) }
        && error.code() != windows::Win32::Foundation::ERROR_MORE_DATA.to_hresult()
    {
        return Err(error.to_string());
    }
    let mut buffer = vec![0u16; size as usize];
    // SAFETY: 宽字缓冲足够
    unsafe {
        GetComputerNameExW(
            ComputerNameDnsHostname,
            Some(PWSTR(buffer.as_mut_ptr())),
            &mut size,
        )
    }
    .map_err(|error| error.to_string())?;
    String::from_utf16(&buffer[..size as usize]).map_err(|error| error.to_string())
}

fn architecture() -> Result<String, String> {
    let mut info = SYSTEM_INFO::default();
    // SAFETY: 独占结构可写
    unsafe { GetNativeSystemInfo(&mut info) };
    // SAFETY: 原生填写该项
    match unsafe { info.Anonymous.Anonymous.wProcessorArchitecture } {
        PROCESSOR_ARCHITECTURE_AMD64 => Ok("x64".to_owned()),
        PROCESSOR_ARCHITECTURE_ARM64 => Ok("arm64".to_owned()),
        PROCESSOR_ARCHITECTURE_INTEL => Ok("ia32".to_owned()),
        value => Err(format!("Unsupported processor architecture: {}", value.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::product_name;

    #[test]
    fn corrects_client_generation_only() {
        assert_eq!(
            product_name("Windows 10 Pro".into(), 22631, true),
            "Windows 11 Pro"
        );
        assert_eq!(
            product_name("Windows 10 Pro".into(), 19045, true),
            "Windows 10 Pro"
        );
        assert_eq!(
            product_name("Windows Server 2025".into(), 26100, false),
            "Windows Server 2025"
        );
    }
}
