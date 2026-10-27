use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_ERROR_NOT_FOUND, IDXGIFactory1,
};

/** 读取显卡名称 */
pub(super) fn read_graphics() -> Result<String, String> {
    // SAFETY: 接口类型匹配
    let factory = unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }
        .map_err(|error| format!("CreateDXGIFactory1: {error}"))?;
    let mut names = Vec::new();
    let mut index = 0;
    loop {
        // SAFETY: 工厂引用有效
        let adapter = match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => adapter,
            Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(error) => return Err(format!("EnumAdapters1: {error}")),
        };
        index += 1;
        // SAFETY: 适配引用有效
        let desc = unsafe { adapter.GetDesc1() }.map_err(|error| error.to_string())?;
        if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        let length = desc
            .Description
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(desc.Description.len());
        let name =
            String::from_utf16(&desc.Description[..length]).map_err(|error| error.to_string())?;
        if name.trim().is_empty() {
            return Err("Empty hardware adapter name".to_owned());
        }
        names.push(name);
    }
    Ok(names.join("; "))
}
