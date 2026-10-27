mod cpu;
mod firmware;
mod graphics;
mod memory;
mod network;
mod power;
mod registry;
mod screen;
mod storage;
mod system;

use crate::types::DeviceInfo;
use windows::Win32::System::SystemInformation::GetTickCount64;

/** 采集完整信息 */
pub fn read_info() -> Result<DeviceInfo, String> {
    let system = system::read_system()?;
    let firmware = firmware::read_firmware()?;
    let did = firmware.did()?;
    let (cpu, cores) = cpu::read_cpu()?;
    let (memory, free_memory) = memory::read_memory()?;
    let network = network::read_network()?;
    let (screen_width, screen_height, scale_factor) = screen::read_screen()?;
    Ok(DeviceInfo {
        did,
        uuid: system::read_identity()?,
        manufacturer: firmware.manufacturer,
        model: firmware.model,
        os: "win".to_owned(),
        os_name: system.name,
        release: system.release,
        kernel: "Windows_NT".to_owned(),
        host: system.hostname,
        arch: system.arch,
        // SAFETY: 无参无指针
        uptime: unsafe { GetTickCount64() } as f64 / 1000.0,
        cpu,
        cores,
        gpu: graphics::read_graphics()?,
        memory,
        free_memory,
        disk: storage::read_disk()?,
        mac: network.mac,
        ip: network.ip,
        net_name: network.name,
        net_type: network.kind,
        screen_width,
        screen_height,
        screen_scale: scale_factor,
        battery: power::read_battery()?,
    })
}
