use std::{
    mem::{MaybeUninit, size_of},
    net::{Ipv4Addr, Ipv6Addr},
    ptr, slice,
};
use windows::Win32::{
    Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_NO_DATA, WIN32_ERROR},
    NetworkManagement::{
        IpHelper::{
            FreeMibTable, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
            GetAdaptersAddresses, GetIpForwardTable2, IF_TYPE_SOFTWARE_LOOPBACK,
            IP_ADAPTER_ADDRESSES_LH, MIB_IPFORWARD_ROW2, MIB_IPFORWARD_TABLE2,
        },
        Ndis::IfOperStatusUp,
    },
    Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6},
};

pub(super) struct Network {
    pub name: String,
    pub mac: String,
    pub ip: Option<String>,
    pub kind: Option<String>,
}

struct NetworkInfo {
    index: u32,
    ipv6_index: u32,
    name: String,
    mac: Option<String>,
    ipv4: Vec<String>,
    ipv6: Vec<String>,
    is_up: bool,
    is_loopback: bool,
    kind: u32,
    ipv6_metric: u32,
}

/** 读取网络状态 */
pub(super) fn read_network() -> Result<Network, String> {
    let mut adapters = filter_adapters(adapters()?);
    let default_index = default_route(&adapters)?;
    adapters.sort_by_key(|(adapter, _)| (Some(adapter.index) != default_index, adapter.index));
    let selected = adapters.into_iter().map(|(value, _)| value).next();
    Ok(match selected {
        Some(value) => Network {
            name: value.name,
            mac: value.mac.unwrap_or_default(),
            ip: value
                .ipv4
                .into_iter()
                .next()
                .or_else(|| value.ipv6.into_iter().next()),
            kind: Some(interface_type(value.kind)),
        },
        None => Network {
            name: String::new(),
            mac: String::new(),
            ip: None,
            kind: None,
        },
    })
}

fn filter_adapters(mut adapters: Vec<(NetworkInfo, u32)>) -> Vec<(NetworkInfo, u32)> {
    adapters.retain(|(adapter, _)| adapter.is_up && !adapter.is_loopback);
    if adapters.iter().any(|(adapter, _)| adapter.mac.is_some()) {
        adapters.retain(|(adapter, _)| adapter.mac.is_some());
    }
    adapters
}

fn physical_address(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty()
        || bytes.iter().all(|value| *value == 0)
        || bytes.iter().all(|value| *value == 255)
    {
        return None;
    }
    Some(
        bytes
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect::<Vec<_>>()
            .join(":"),
    )
}

fn interface_type(kind: u32) -> String {
    match kind {
        6 => "Ethernet".to_owned(),
        71 => "Wi-Fi".to_owned(),
        value => format!("ifType:{value}"),
    }
}

fn adapters() -> Result<Vec<(NetworkInfo, u32)>, String> {
    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size = 0;
    loop {
        let mut buffer = vec![
            MaybeUninit::<IP_ADAPTER_ADDRESSES_LH>::uninit();
            (size as usize).div_ceil(size_of::<IP_ADAPTER_ADDRESSES_LH>())
        ];
        let pointer = if buffer.is_empty() {
            None
        } else {
            Some(buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>())
        };
        // SAFETY: 原生结构对齐
        // 缓冲容量足够
        // 空值仅查长度
        let status = WIN32_ERROR(unsafe {
            GetAdaptersAddresses(u32::from(AF_UNSPEC.0), flags, None, pointer, &mut size)
        });
        if status == ERROR_BUFFER_OVERFLOW {
            continue;
        }
        if status == ERROR_NO_DATA {
            return Ok(Vec::new());
        }
        status.ok().map_err(|error| error.to_string())?;
        let mut current = pointer.unwrap_or(ptr::null_mut());
        let mut list = Vec::new();
        while !current.is_null() {
            // SAFETY: 原生链项对齐
            // 缓冲持续存活
            let adapter = unsafe { &*current };
            let mut ipv4 = Vec::new();
            let mut ipv6 = Vec::new();
            let mut address = adapter.FirstUnicastAddress;
            while !address.is_null() {
                // SAFETY: 原生链项对齐
                // 缓冲持续存活
                let socket = unsafe { &(*address).Address };
                if !socket.lpSockaddr.is_null()
                    && socket.iSockaddrLength
                        >= size_of::<windows::Win32::Networking::WinSock::SOCKADDR>() as i32
                {
                    // SAFETY: 地址头已验界
                    match unsafe { ptr::addr_of!((*socket.lpSockaddr).sa_family).read_unaligned() }
                    {
                        AF_INET if socket.iSockaddrLength >= size_of::<SOCKADDR_IN>() as i32 => {
                            // SAFETY: IPv4长度已验
                            let value =
                                unsafe { socket.lpSockaddr.cast::<SOCKADDR_IN>().read_unaligned() };
                            // SAFETY: IPv4成员有效
                            ipv4.push(
                                Ipv4Addr::from(unsafe { value.sin_addr.S_un.S_addr }.to_ne_bytes())
                                    .to_string(),
                            );
                        }
                        AF_INET6 if socket.iSockaddrLength >= size_of::<SOCKADDR_IN6>() as i32 => {
                            // SAFETY: IPv6长度已验
                            let value = unsafe {
                                socket.lpSockaddr.cast::<SOCKADDR_IN6>().read_unaligned()
                            };
                            // SAFETY: IPv6字节有效
                            let ip = Ipv6Addr::from(unsafe { value.sin6_addr.u.Byte });
                            // SAFETY: IPv6域号有效
                            let scope = unsafe { value.Anonymous.sin6_scope_id };
                            ipv6.push(if scope == 0 {
                                ip.to_string()
                            } else {
                                format!("{ip}%{scope}")
                            });
                        }
                        _ => {}
                    }
                }
                // SAFETY: 原生链项有效
                // 缓冲持续存活
                address = unsafe { (*address).Next };
            }
            let length = adapter.PhysicalAddressLength as usize;
            if length > adapter.PhysicalAddress.len() {
                return Err("Invalid adapter address length".to_owned());
            }
            let mac = physical_address(&adapter.PhysicalAddress[..length]);
            // SAFETY: 原生填写该项
            let index = unsafe { adapter.Anonymous1.Anonymous.IfIndex };
            if adapter.FriendlyName.is_null() {
                return Err("Missing adapter text".to_owned());
            }
            list.push((
                NetworkInfo {
                    index: if index == 0 {
                        adapter.Ipv6IfIndex
                    } else {
                        index
                    },
                    // SAFETY: 原生宽串终止
                    // 非空且仍存活
                    name: unsafe { adapter.FriendlyName.to_string() }
                        .map_err(|error| error.to_string())?,
                    ipv6_index: adapter.Ipv6IfIndex,
                    ipv6_metric: adapter.Ipv6Metric,
                    kind: adapter.IfType,
                    mac,
                    ipv4,
                    ipv6,
                    is_up: adapter.OperStatus == IfOperStatusUp,
                    is_loopback: adapter.IfType == IF_TYPE_SOFTWARE_LOOPBACK,
                },
                adapter.Ipv4Metric,
            ));
            current = adapter.Next;
        }
        return Ok(list);
    }
}

struct RouteTable(*mut MIB_IPFORWARD_TABLE2);

impl Drop for RouteTable {
    fn drop(&mut self) {
        // SAFETY: 原生表独占权
        // 仅此释放一次
        unsafe { FreeMibTable(self.0.cast()) };
    }
}

fn default_route(adapters: &[(NetworkInfo, u32)]) -> Result<Option<u32>, String> {
    let mut pointer = ptr::null_mut();
    // SAFETY: 独占指针可写
    // 成功交接表权
    unsafe { GetIpForwardTable2(AF_UNSPEC, &mut pointer) }
        .ok()
        .map_err(|error| error.to_string())?;
    if pointer.is_null() {
        return Err("Missing route table".to_owned());
    }
    let table = RouteTable(pointer);
    // SAFETY: 成功原生分配
    let count = unsafe { (*table.0).NumEntries } as usize;
    // SAFETY: 原生表项对齐
    // 表长由其提供
    // 表权覆盖借用
    let rows: &[MIB_IPFORWARD_ROW2] =
        unsafe { slice::from_raw_parts(ptr::addr_of!((*table.0).Table).cast(), count) };
    let candidates = rows.iter().filter_map(|row| {
        // SAFETY: 原生地址族值
        let family = unsafe { row.DestinationPrefix.Prefix.si_family };
        if row.DestinationPrefix.PrefixLength != 0
            || row.ValidLifetime == 0
            || row.Loopback
            || ![AF_INET, AF_INET6].contains(&family)
        {
            return None;
        }
        // SAFETY: 地址类型已验
        let is_default = unsafe {
            if family == AF_INET {
                row.DestinationPrefix.Prefix.Ipv4.sin_addr.S_un.S_addr == 0
            } else {
                row.DestinationPrefix.Prefix.Ipv6.sin6_addr.u.Byte == [0; 16]
            }
        };
        if !is_default {
            return None;
        }
        adapters
            .iter()
            .find(|(adapter, _)| {
                (if family == AF_INET {
                    adapter.index
                } else {
                    adapter.ipv6_index
                }) == row.InterfaceIndex
                    && adapter.is_up
                    && !adapter.is_loopback
            })
            .map(|(adapter, metric)| {
                (
                    adapter.index,
                    row.Metric,
                    if family == AF_INET {
                        *metric
                    } else {
                        adapter.ipv6_metric
                    },
                )
            })
    });
    Ok(select_route(candidates))
}

fn select_route(routes: impl Iterator<Item = (u32, u32, u32)>) -> Option<u32> {
    routes
        .min_by_key(|(index, route, interface)| (u64::from(*route) + u64::from(*interface), *index))
        .map(|(index, _, _)| index)
}

#[cfg(test)]
mod tests {
    use super::{NetworkInfo, filter_adapters, physical_address, select_route};

    fn adapter(
        index: u32,
        mac: Option<&str>,
        is_up: bool,
        is_loopback: bool,
    ) -> (NetworkInfo, u32) {
        (
            NetworkInfo {
                index,
                ipv6_index: index,
                name: format!("adapter-{index}"),
                mac: mac.map(str::to_owned),
                ipv4: Vec::new(),
                ipv6: Vec::new(),
                is_up,
                is_loopback,
                kind: 6,
                ipv6_metric: 0,
            },
            0,
        )
    }

    #[test]
    fn prefers_mac_over_addressless_tunnel() {
        let values = filter_adapters(vec![
            adapter(1, None, true, false),
            adapter(2, Some("02:11:22:33:44:55"), true, false),
        ]);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].0.index, 2);
    }

    #[test]
    fn inactive_or_loopback_mac_does_not_hide_active_tunnel() {
        let values = filter_adapters(vec![
            adapter(1, None, true, false),
            adapter(2, Some("02:11:22:33:44:55"), false, false),
            adapter(3, Some("02:11:22:33:44:66"), true, true),
        ]);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].0.index, 1);
        assert!(filter_adapters(vec![adapter(4, None, false, false)]).is_empty());
    }

    #[test]
    fn keeps_all_active_mac_candidates_for_route_selection() {
        let values = filter_adapters(vec![
            adapter(3, Some("02:11:22:33:44:55"), true, false),
            adapter(2, Some("02:11:22:33:44:66"), true, false),
        ]);
        assert_eq!(values.len(), 2);
        assert_eq!(
            select_route(
                values
                    .iter()
                    .map(|(value, metric)| (value.index, 0, *metric))
            ),
            Some(2)
        );
    }

    #[test]
    fn rejects_placeholder_mac_addresses() {
        assert!(physical_address(&[]).is_none());
        assert!(physical_address(&[0; 6]).is_none());
        assert!(physical_address(&[255; 6]).is_none());
        assert_eq!(
            physical_address(&[2, 17, 34, 51, 68, 85]).as_deref(),
            Some("02:11:22:33:44:55")
        );
    }

    #[test]
    fn combined_metrics() {
        assert_eq!(select_route([(1, 1, 90), (2, 20, 5)].into_iter()), Some(2));
    }

    #[test]
    fn no_default_route() {
        assert_eq!(select_route(std::iter::empty()), None);
    }

    #[test]
    fn metrics_do_not_overflow() {
        assert_eq!(
            select_route([(1, u32::MAX, 1), (2, 10, 10)].into_iter()),
            Some(2)
        );
    }

    #[test]
    fn stable_metric_ties() {
        assert_eq!(select_route([(3, 10, 5), (2, 5, 10)].into_iter()), Some(2));
    }
}
