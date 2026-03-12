#[cfg(not(windows))]
use chrono::TimeZone;
use chrono::{DateTime, Local};

pub struct Clock;

impl Clock {
    pub fn get() -> DateTime<Local> {
        Local::now()
    }

    #[cfg(not(windows))]
    pub fn set<T: TimeZone>(t: DateTime<T>) {
        use libc::{settimeofday, suseconds_t, time_t, timeval, timezone};
        let t = t.with_timezone(&Local);
        let mut tv: timeval = unsafe { std::mem::zeroed() };
        tv.tv_sec = t.timestamp() as time_t;
        tv.tv_usec = t.timestamp_subsec_micros() as suseconds_t;
        unsafe {
            let mock_tz: *const timezone = std::ptr::null();
            settimeofday(&tv as *const timeval, mock_tz);
        }
    }

    #[cfg(windows)]
    pub fn set<T: TimeZone>(t: DateTime<T>) {
        use chrono::Weekday;
        use winapi::shared::minwindef::WORD;
        use winapi::um::minwinbase::SYSTEMTIME;
        use winapi::um::sysinfoapi::SetSystemTime;

        let t = t.with_timezone(&Local);

        let dow: WORD = match t.weekday() {
            Weekday::Mon => 1,
            Weekday::Tue => 2,
            Weekday::Wed => 3,
            Weekday::Thu => 4,
            Weekday::Fri => 5,
            Weekday::Sat => 6,
            Weekday::Sun => 0,
        };

        let mut ns = t.nanosecond();
        if ns > 1_000_000_000 {
            ns -= 1_000_000_000;
        }

        let systime = SYSTEMTIME {
            wYear: t.year() as WORD,
            wMonth: t.month() as WORD,
            wDayOfWeek: dow,
            wDay: t.day() as WORD,
            wHour: t.hour() as WORD,
            wMinute: t.minute() as WORD,
            wSecond: t.second() as WORD,
            wMilliseconds: (ns / 1_000_000) as WORD,
        };

        unsafe {
            SetSystemTime(&systime as *const SYSTEMTIME);
        }
    }
}
