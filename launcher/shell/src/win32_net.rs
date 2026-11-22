use windows_sys::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetIfTable, IF_OPER_STATUS_CONNECTED, IF_OPER_STATUS_OPERATIONAL, MIB_IFROW, MIB_IFTABLE,
};

#[derive(Debug, Clone)]
pub struct ActiveAdapterInfo {
    pub adapter: String,
    pub adapter_type: &'static str,
    pub mac: String,
    pub link_speed_mbps: u64,
    pub ipv4: String,
    pub connected: bool,
}

const IF_TYPE_ETHERNET_CSMACD: u32 = 6;
const IF_TYPE_SOFTWARE_LOOPBACK: u32 = 24;
const IF_TYPE_IEEE80211: u32 = 71;

pub fn query_active_adapter() -> ActiveAdapterInfo {
    let ipv4 = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|sock| {
            sock.connect("8.8.8.8:80")?;
            sock.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "—".to_string());
    let connected = ipv4 != "—";

    let mut size: u32 = 0;
    let first = unsafe { GetIfTable(std::ptr::null_mut(), &mut size, 0) };
    if first != ERROR_INSUFFICIENT_BUFFER || size == 0 {
        return ActiveAdapterInfo {
            adapter: if connected { "Network".to_string() } else { "Offline".to_string() },
            adapter_type: if connected { "ethernet" } else { "offline" },
            mac: "—".to_string(),
            link_speed_mbps: 0,
            ipv4,
            connected,
        };
    }

    let mut buffer = vec![0u8; size as usize];
    let table = buffer.as_mut_ptr() as *mut MIB_IFTABLE;
    if unsafe { GetIfTable(table, &mut size, 0) } != 0 {
        return ActiveAdapterInfo {
            adapter: if connected { "Network".to_string() } else { "Offline".to_string() },
            adapter_type: if connected { "ethernet" } else { "offline" },
            mac: "—".to_string(),
            link_speed_mbps: 0,
            ipv4,
            connected,
        };
    }

    let count = unsafe { (*table).dwNumEntries as usize };
    let rows_ptr = unsafe { (*table).table.as_ptr() };
    let rows = unsafe { std::slice::from_raw_parts(rows_ptr, count) };

    let best: Option<&MIB_IFROW> = rows
        .iter()
        .filter(|row| {
            row.dwType != IF_TYPE_SOFTWARE_LOOPBACK
                && (row.dwOperStatus == IF_OPER_STATUS_OPERATIONAL
                    || row.dwOperStatus == IF_OPER_STATUS_CONNECTED)
                && (row.dwInOctets > 0 || row.dwOutOctets > 0)
        })
        .max_by_key(|row| {
            let type_bonus: u64 = match row.dwType {
                IF_TYPE_ETHERNET_CSMACD => 2_000_000_000_000,
                IF_TYPE_IEEE80211 => 1_000_000_000_000,
                _ => 0,
            };
            type_bonus + (row.dwInOctets as u64) + (row.dwOutOctets as u64)
        });

    if let Some(row) = best {
        let descr_len = (row.dwDescrLen as usize).min(row.bDescr.len());
        let raw_descr = &row.bDescr[..descr_len];
        let trimmed = raw_descr
            .iter()
            .copied()
            .take_while(|&b| b != 0)
            .collect::<Vec<u8>>();
        let adapter = String::from_utf8_lossy(&trimmed).trim().to_string();

        let mac_len = (row.dwPhysAddrLen as usize).min(row.bPhysAddr.len());
        let mac = if mac_len > 0 {
            row.bPhysAddr[..mac_len]
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(":")
        } else {
            "—".to_string()
        };

        let adapter_type = match row.dwType {
            IF_TYPE_IEEE80211 => "wifi",
            IF_TYPE_ETHERNET_CSMACD => "ethernet",
            _ => "other",
        };
        let link_speed_mbps = (row.dwSpeed as u64) / 1_000_000;

        return ActiveAdapterInfo {
            adapter: if adapter.is_empty() { "Active Adapter".to_string() } else { adapter },
            adapter_type,
            mac,
            link_speed_mbps,
            ipv4,
            connected,
        };
    }

    ActiveAdapterInfo {
        adapter: if connected { "Network".to_string() } else { "Offline".to_string() },
        adapter_type: if connected { "ethernet" } else { "offline" },
        mac: "—".to_string(),
        link_speed_mbps: 0,
        ipv4,
        connected,
    }
}
