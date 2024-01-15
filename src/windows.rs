use std::{
    mem::MaybeUninit,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    os::windows::fs::MetadataExt,
};
use windows::Win32::{
    Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS, SYSTEMTIME},
    NetworkManagement::{
        IpHelper::{GetAdaptersAddresses, GET_ADAPTERS_ADDRESSES_FLAGS, IP_ADAPTER_ADDRESSES_LH},
        Ndis::IfOperStatusUp,
    },
    Networking::WinSock::{AF_INET, AF_INET6, SOCKADDR_IN, SOCKADDR_IN6},
    System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime},
};

pub trait MetadataExtModified {
    fn modified_date(&self) -> std::io::Result<String>;
}

impl MetadataExtModified for std::fs::Metadata {
    fn modified_date(&self) -> std::io::Result<String> {
        let file_time = self.last_write_time();

        let local_time = || -> windows::core::Result<SYSTEMTIME> {
            let mut utc = MaybeUninit::uninit();
            let mut local = MaybeUninit::uninit();
            unsafe {
                FileTimeToSystemTime((&file_time as *const u64).cast(), utc.as_mut_ptr())?;
                SystemTimeToTzSpecificLocalTime(None, utc.as_ptr(), local.as_mut_ptr())?;
                Ok(local.assume_init())
            }
        };

        Ok(system_time_to_string(
            local_time().map_err(|_| std::io::Error::last_os_error())?,
        ))
    }
}

fn system_time_to_string(system_time: SYSTEMTIME) -> String {
    let month = match system_time.wMonth {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "Unk",
    };
    let mut hour = system_time.wHour;
    let suffix = match hour {
        0 => {
            hour += 12;
            "AM"
        }
        1..=11 => "AM",
        12 => "PM",
        _ => {
            hour -= 12;
            "PM"
        }
    };
    format!(
        "{:02}-{}-{} {}:{:02} {}",
        system_time.wDay, month, system_time.wYear, hour, system_time.wMinute, suffix
    )
}

struct IpAdapterAddresses {
    buffer: *mut u8,
    req_bytes: u32,
}

impl Drop for IpAdapterAddresses {
    fn drop(&mut self) {
        let layout = IpAdapterAddresses::layout(self.req_bytes);
        unsafe {
            if !self.buffer.is_null() {
                std::alloc::dealloc(self.buffer, layout);
            }
        }
    }
}

impl IpAdapterAddresses {
    fn new() -> Self {
        Self {
            buffer: std::ptr::null_mut(),
            req_bytes: 0,
        }
    }

    fn as_inner(&mut self) -> *mut IP_ADAPTER_ADDRESSES_LH {
        self.buffer.cast()
    }

    fn set_req_bytes(&mut self, req_bytes: u32) {
        self.req_bytes = req_bytes;
        let layout = IpAdapterAddresses::layout(self.req_bytes);
        unsafe {
            if !self.buffer.is_null() {
                std::alloc::dealloc(self.buffer, layout);
            }
            self.buffer = std::alloc::alloc(layout);
        }
    }

    fn layout(req_bytes: u32) -> std::alloc::Layout {
        // Copied from std
        const fn div_ceil(lhs: usize, rhs: usize) -> usize {
            let d = lhs / rhs;
            let r = lhs % rhs;
            if r > 0 && rhs > 0 {
                d + 1
            } else {
                d
            }
        }

        const STRUCT_SIZE: usize = std::mem::size_of::<IP_ADAPTER_ADDRESSES_LH>();

        let count = div_ceil(req_bytes as _, STRUCT_SIZE);

        std::alloc::Layout::array::<IP_ADAPTER_ADDRESSES_LH>(count)
            .expect("Unable to construct layout")
    }
}

/// Returns the IP address of the first network interface that is up/enabled.
pub fn default_ip_address(use_ipv4: bool) -> std::io::Result<IpAddr> {
    let family = if use_ipv4 { AF_INET } else { AF_INET6 };

    let mut ip_adapter_addresses = IpAdapterAddresses::new();
    let mut req_bytes = 0;
    let ret = unsafe {
        GetAdaptersAddresses(
            family.0 as _,
            GET_ADAPTERS_ADDRESSES_FLAGS(0),
            None,
            None,
            &mut req_bytes,
        )
    };

    if ret != ERROR_BUFFER_OVERFLOW.0 {
        return Err(std::io::Error::last_os_error());
    }

    ip_adapter_addresses.set_req_bytes(req_bytes);

    let ret = unsafe {
        GetAdaptersAddresses(
            family.0 as _,
            GET_ADAPTERS_ADDRESSES_FLAGS(0),
            None,
            Some(ip_adapter_addresses.as_inner()),
            &mut req_bytes,
        )
    };

    if ret != ERROR_SUCCESS.0 {
        return Err(std::io::Error::last_os_error());
    }

    unsafe {
        let mut curr_addr = ip_adapter_addresses.as_inner();
        while !curr_addr.is_null() {
            if (*curr_addr).OperStatus == IfOperStatusUp {
                let unicast_addr = (*curr_addr).FirstUnicastAddress;
                let socket_addr = (*unicast_addr).Address;
                let addr = socket_addr.lpSockaddr;
                match (*addr).sa_family {
                    AF_INET if use_ipv4 => {
                        let addr: *mut SOCKADDR_IN = addr as *mut SOCKADDR_IN;
                        let addr_bytes = (*addr).sin_addr.S_un.S_addr;
                        return Ok(IpAddr::V4(Ipv4Addr::from(addr_bytes.swap_bytes())));
                    }
                    AF_INET6 if !use_ipv4 => {
                        let sockaddr: *mut SOCKADDR_IN6 = addr as *mut SOCKADDR_IN6;
                        let addr_bytes = (*sockaddr).sin6_addr.u.Byte;
                        return Ok(IpAddr::V6(Ipv6Addr::from(addr_bytes)));
                    }
                    _ => (),
                }
            }
            curr_addr = (*curr_addr).Next;
        }
    }

    Ok(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}
