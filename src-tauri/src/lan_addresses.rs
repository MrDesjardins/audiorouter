//! This PC's private IPv4 addresses, offered as HTTP API network listeners
//! (HTTP-09). Only connected, non-loopback adapters are listed.
use std::net::Ipv4Addr;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LanAddress {
    pub address: Ipv4Addr,
    pub adapter: String,
}

#[cfg(windows)]
pub fn list() -> Result<Vec<LanAddress>, String> {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
        GAA_FLAG_SKIP_MULTICAST, IF_TYPE_SOFTWARE_LOOPBACK, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};
    const ERROR_BUFFER_OVERFLOW: u32 = 111;
    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size = 16 * 1024u32;
    for _ in 0..3 {
        // u64 storage keeps the adapter records 8-byte aligned.
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        let head = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        // SAFETY: `head` points to the live, 8-byte-aligned `buffer`, which
        // holds at least `size` bytes; the call writes at most that many and
        // updates `size` when it needs more. No reserved pointer is passed.
        let status = unsafe {
            GetAdaptersAddresses(u32::from(AF_INET.0), flags, None, Some(head), &mut size)
        };
        if status == ERROR_BUFFER_OVERFLOW {
            continue;
        }
        if status != 0 {
            return Err(format!("Cannot list network adapters (error {status})"));
        }
        let mut found = Vec::new();
        let mut adapter = head.cast_const();
        // Every pointer below points into `buffer`, which outlives the walk:
        // the adapter list, its unicast lists and the friendly-name strings
        // are all written by GetAdaptersAddresses into that one allocation
        // and terminated by null `Next` pointers.
        while !adapter.is_null() {
            // SAFETY: `adapter` is non-null and points to an IP_ADAPTER_ADDRESSES_LH
            // record inside `buffer` (the head, or a `Next` link written by the call).
            let record = unsafe { &*adapter };
            if record.OperStatus == IfOperStatusUp && record.IfType != IF_TYPE_SOFTWARE_LOOPBACK {
                // SAFETY: FriendlyName is a NUL-terminated wide string inside `buffer`.
                let name = unsafe { record.FriendlyName.to_string() }.unwrap_or_default();
                let mut unicast = record.FirstUnicastAddress.cast_const();
                while !unicast.is_null() {
                    // SAFETY: `unicast` is a non-null link to a unicast record inside `buffer`.
                    let entry = unsafe { &*unicast };
                    let socket = entry.Address;
                    if !socket.lpSockaddr.is_null()
                        && socket.iSockaddrLength as usize >= std::mem::size_of::<SOCKADDR_IN>()
                        // SAFETY: `lpSockaddr` was checked non-null and its length covers a full
                        // SOCKADDR_IN, so reading the family field is in bounds.
                        && unsafe { (*socket.lpSockaddr).sa_family } == AF_INET
                    {
                        // AF_INET with a full SOCKADDR_IN length; the union's
                        // S_addr holds the address in network byte order.
                        // SAFETY: the family is AF_INET and the length covers SOCKADDR_IN, so the
                        // cast pointer addresses a complete SOCKADDR_IN inside `buffer`.
                        let raw = unsafe {
                            (*socket.lpSockaddr.cast::<SOCKADDR_IN>())
                                .sin_addr
                                .S_un
                                .S_addr
                        };
                        let address = Ipv4Addr::from(raw.to_ne_bytes());
                        if crate::http_api::lan_address_allowed(address) {
                            found.push(LanAddress {
                                address,
                                adapter: name.clone(),
                            });
                        }
                    }
                    unicast = entry.Next.cast_const();
                }
            }
            adapter = record.Next.cast_const();
        }
        found.sort_by_key(|entry| entry.address);
        found.dedup_by_key(|entry| entry.address);
        return Ok(found);
    }
    Err("Cannot list network adapters (list kept growing)".into())
}

#[cfg(not(windows))]
pub fn list() -> Result<Vec<LanAddress>, String> {
    Ok(Vec::new())
}
