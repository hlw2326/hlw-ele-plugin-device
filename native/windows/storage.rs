use std::mem::{offset_of, size_of};
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_MORE_DATA, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE,
            GetVolumePathNameW, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, OPEN_EXISTING,
        },
        System::{
            IO::DeviceIoControl,
            Ioctl::{
                DISK_EXTENT, DISK_GEOMETRY_EX, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                VOLUME_DISK_EXTENTS,
            },
            SystemInformation::GetWindowsDirectoryW,
        },
    },
    core::PCWSTR,
};

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: 句柄独占有效
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn open(path: &str) -> Result<Handle, String> {
    let path = path.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    // SAFETY: 路径零值终止
    unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map(Handle)
    .map_err(|error| format!("CreateFileW(storage): {error}"))
}

fn control(handle: &Handle, code: u32) -> Result<(Vec<u64>, usize), String> {
    let mut buffer = vec![0u64; 512];
    loop {
        let mut returned = 0;
        // SAFETY: 缓冲对齐足够
        let result = unsafe {
            DeviceIoControl(
                handle.0,
                code,
                None,
                0,
                Some(buffer.as_mut_ptr().cast()),
                (buffer.len() * 8) as u32,
                Some(&mut returned),
                None,
            )
        };
        match result {
            Ok(()) if returned as usize <= buffer.len() * 8 => {
                return Ok((buffer, returned as usize));
            }
            Ok(()) => return Err("Invalid storage response length".to_owned()),
            Err(error) if error.code() == ERROR_MORE_DATA.to_hresult() && buffer.len() < 131072 => {
                buffer.resize(buffer.len() * 2, 0);
            }
            Err(error) => return Err(format!("DeviceIoControl(storage): {error}")),
        }
    }
}

/** 读取系统磁盘 */
pub(super) fn read_disk() -> Result<f64, String> {
    let mut path = vec![0u16; 32768];
    // SAFETY: 缓冲独占有效
    let length = unsafe { GetWindowsDirectoryW(Some(&mut path)) } as usize;
    if length == 0 || length >= path.len() {
        return Err(format!(
            "GetWindowsDirectoryW: {}",
            windows::core::Error::from_thread()
        ));
    }
    let mut root = vec![0u16; 32768];
    // SAFETY: 路径缓冲有效
    unsafe { GetVolumePathNameW(PCWSTR(path.as_ptr()), &mut root) }
        .map_err(|error| error.to_string())?;
    let length = root
        .iter()
        .position(|value| *value == 0)
        .ok_or("Unterminated volume path")?;
    let root = String::from_utf16(&root[..length]).map_err(|error| error.to_string())?;
    if root.len() != 3 || !root.ends_with(":\\") {
        return Err("System volume must have a drive-letter root".to_owned());
    }
    let volume = open(&format!("\\\\.\\{}", &root[..2]))?;
    let (buffer, length) = control(&volume, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS)?;
    let offset = offset_of!(VOLUME_DISK_EXTENTS, Extents);
    if length < offset {
        return Err("Truncated volume extents".to_owned());
    }
    // SAFETY: 长度对齐已验
    let count = unsafe { buffer.as_ptr().cast::<u32>().read() } as usize;
    if count == 0 || count > (length - offset) / size_of::<DISK_EXTENT>() {
        return Err("Invalid volume extents".to_owned());
    }
    // SAFETY: 全部记录已验
    let extents = unsafe {
        std::slice::from_raw_parts(
            buffer
                .as_ptr()
                .cast::<u8>()
                .add(offset)
                .cast::<DISK_EXTENT>(),
            count,
        )
    };
    let disk = extents[0].DiskNumber;
    if extents.iter().any(|value| value.DiskNumber != disk) {
        return Err("System volume spans multiple physical disks".to_owned());
    }
    let physical = open(&format!("\\\\.\\PhysicalDrive{disk}"))?;
    let (buffer, length) = control(&physical, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX)?;
    if length < offset_of!(DISK_GEOMETRY_EX, Data) {
        return Err("Truncated disk geometry".to_owned());
    }
    // SAFETY: 字段边界已验
    let size = unsafe {
        buffer
            .as_ptr()
            .cast::<u8>()
            .add(offset_of!(DISK_GEOMETRY_EX, DiskSize))
            .cast::<i64>()
            .read()
    };
    if size <= 0 {
        return Err("Invalid physical disk size".to_owned());
    }
    Ok(size as f64)
}
