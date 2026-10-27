use sha2::{Digest, Sha256};
use windows::Win32::System::SystemInformation::{GetSystemFirmwareTable, RSMB};

#[derive(Default)]
pub(super) struct Firmware {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub uuid: Option<String>,
}

impl Firmware {
    /** 获取硬件标识 */
    pub(super) fn did(&self) -> Result<String, String> {
        let uuid = self.uuid.as_deref().ok_or("Missing SMBIOS Type 1 UUID")?;
        Ok(format!("{:x}", Sha256::digest(uuid.as_bytes())))
    }
}

/** 采集固件信息 */
pub(super) fn read_firmware() -> Result<Firmware, String> {
    parse_table(&read_table()?)
}

fn read_table() -> Result<Vec<u8>, String> {
    // SAFETY: 空缓冲查长度
    let length = unsafe { GetSystemFirmwareTable(RSMB, 0, None) };
    if length == 0 {
        return Err(format!(
            "GetSystemFirmwareTable: {}",
            windows::core::Error::from_thread()
        ));
    }
    let mut buffer = vec![0u8; length as usize];
    loop {
        // SAFETY: 缓冲独占有效
        let length = unsafe { GetSystemFirmwareTable(RSMB, 0, Some(&mut buffer)) };
        if length == 0 {
            return Err(format!(
                "GetSystemFirmwareTable: {}",
                windows::core::Error::from_thread()
            ));
        }
        if length as usize <= buffer.len() {
            buffer.truncate(length as usize);
            return Ok(buffer);
        }
        buffer.resize(length as usize, 0);
    }
}

fn parse_table(buffer: &[u8]) -> Result<Firmware, String> {
    let header = buffer.get(..8).ok_or("Truncated RawSMBIOSData header")?;
    let length = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    let table = buffer
        .get(8..)
        .and_then(|value| value.get(..length))
        .ok_or("Truncated SMBIOS table")?;
    let mut offset = 0;
    while offset < table.len() {
        let record = &table[offset..];
        if record.len() < 4 || record[1] < 4 || record[1] as usize > record.len() {
            return Err("Invalid SMBIOS structure length".to_owned());
        }
        let formatted = &record[..record[1] as usize];
        let tail = &record[formatted.len()..];
        let end = tail
            .windows(2)
            .position(|value| value == [0, 0])
            .ok_or("Unterminated SMBIOS strings")?;
        if record[0] == 1 {
            return Ok(Firmware {
                manufacturer: read_field(formatted, 4, &tail[..end])?,
                model: read_field(formatted, 5, &tail[..end])?,
                uuid: formatted
                    .get(8..24)
                    .and_then(|value| decode_uuid(value, header[1], header[2])),
            });
        }
        if record[0] == 127 {
            break;
        }
        offset += formatted.len() + end + 2;
    }
    Ok(Firmware::default())
}

fn read_field(record: &[u8], offset: usize, strings: &[u8]) -> Result<Option<String>, String> {
    let Some(&index) = record.get(offset) else {
        return Ok(None);
    };
    if index == 0 {
        return Ok(None);
    }
    let bytes = strings
        .split(|value| *value == 0)
        .nth(index as usize - 1)
        .ok_or("Invalid SMBIOS string index")?;
    let value = String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())?;
    Ok((!value.trim().is_empty()).then(|| value.trim().to_owned()))
}

fn decode_uuid(bytes: &[u8], major: u8, minor: u8) -> Option<String> {
    if bytes.iter().all(|value| *value == 0) || bytes.iter().all(|value| *value == 255) {
        return None;
    }
    let mut uuid: [u8; 16] = bytes.try_into().ok()?;
    if (major, minor) >= (2, 6) {
        uuid[..4].reverse();
        uuid[4..6].reverse();
        uuid[6..8].reverse();
    }
    let hex = uuid
        .iter()
        .map(|value| format!("{value:02x}"))
        .collect::<String>();
    Some(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(major: u8, minor: u8) -> Vec<u8> {
        let mut table = vec![1, 24, 0, 0, 1, 2, 0, 0];
        table.extend(0u8..16);
        table.extend_from_slice(b"Maker\0Model\0\0");
        let mut buffer = vec![0, major, minor, 0];
        buffer.extend_from_slice(&(table.len() as u32).to_le_bytes());
        buffer.extend(table);
        buffer
    }

    #[test]
    fn parses_fields_and_uuid_order() {
        let value = parse_table(&fixture(2, 6)).unwrap();
        assert_eq!(value.manufacturer.as_deref(), Some("Maker"));
        assert_eq!(value.model.as_deref(), Some("Model"));
        assert_eq!(
            value.uuid.as_deref(),
            Some("03020100-0504-0706-0809-0a0b0c0d0e0f")
        );
        assert_eq!(
            parse_table(&fixture(2, 5)).unwrap().uuid.as_deref(),
            Some("00010203-0405-0607-0809-0a0b0c0d0e0f")
        );
    }

    #[test]
    fn hashes_normalized_uuid_vectors() {
        for (minor, expected) in [
            (
                6,
                "77bacbcfe81c3a89a2e461023db4c697a6c58206fcd77dd1a31b77928d6de9a9",
            ),
            (
                5,
                "f920f1583dc9da3bc0569e6e1dc5f231b1292cbe378c62c0d88f599a7e68dab1",
            ),
        ] {
            assert_eq!(
                parse_table(&fixture(2, minor)).unwrap().did().unwrap(),
                expected
            );
        }
    }

    #[test]
    fn rejects_corrupt_records() {
        for length in 0..8 {
            assert!(parse_table(&fixture(3, 0)[..length]).is_err());
        }
        let mut buffer = fixture(3, 0);
        buffer.pop();
        assert!(parse_table(&buffer).is_err());
        for length in [0, 3, 255] {
            let mut buffer = fixture(3, 0);
            buffer[9] = length;
            assert!(parse_table(&buffer).is_err());
        }
        let mut buffer = fixture(3, 0);
        buffer[12] = 9;
        assert!(parse_table(&buffer).is_err());
        buffer[12] = 1;
        buffer[32] = 255;
        assert!(parse_table(&buffer).is_err());
    }

    #[test]
    fn rejects_missing_hardware_identity() {
        for fill in [0, 255] {
            let mut buffer = fixture(3, 0);
            buffer[16..32].fill(fill);
            buffer[12..14].fill(0);
            let value = parse_table(&buffer).unwrap();
            assert!(value.model.is_none() && value.manufacturer.is_none());
            assert_eq!(value.did().unwrap_err(), "Missing SMBIOS Type 1 UUID");
        }
        let mut buffer = fixture(3, 0);
        buffer[8] = 127;
        assert!(parse_table(&buffer).unwrap().did().is_err());
        let mut buffer = fixture(3, 0);
        buffer.drain(16..32);
        buffer[9] = 8;
        let length = (buffer.len() - 8) as u32;
        buffer[4..8].copy_from_slice(&length.to_le_bytes());
        assert!(parse_table(&buffer).unwrap().did().is_err());
    }
}
