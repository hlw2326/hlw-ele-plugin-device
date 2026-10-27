#![deny(clippy::undocumented_unsafe_blocks)]

#[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
compile_error!("This release supports Windows x64 only");

#[path = "windows/mod.rs"]
mod platform;
mod types;

use napi_derive::napi;
use types::DeviceInfo;

/** 获取完整信息 */
#[napi(js_name = "info")]
pub fn info() -> napi::Result<DeviceInfo> {
    platform::read_info().map_err(napi::Error::from_reason)
}
