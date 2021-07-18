use winapi::um::minwinbase::SYSTEMTIME;
use winapi::um::timezoneapi::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};

use std::mem::MaybeUninit;
use std::os::windows::fs::MetadataExt;

pub fn get_metadata(metadata: &std::fs::Metadata) -> (u64, u64) {
    (metadata.file_size(), metadata.last_write_time())
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

pub fn get_date(file_time: u64) -> std::io::Result<String> {
    let mut utc = MaybeUninit::uninit();
    let mut local = MaybeUninit::uninit();

    let local_time = unsafe {
        if FileTimeToSystemTime((&file_time as *const u64).cast(), utc.as_mut_ptr()) == 0
            || SystemTimeToTzSpecificLocalTime(std::ptr::null(), utc.as_ptr(), local.as_mut_ptr())
                == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        local.assume_init()
    };

    Ok(system_time_to_string(local_time))
}
