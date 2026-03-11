use chrono::TimeZone;
use chrono::{DateTime, Local};
use clap::{Parser, ValueEnum};

struct Clock;
impl Clock {
    fn get() -> DateTime<Local> {
        Local::now()
    }

    #[cfg(not(windows))]
    fn set<T: TimeZone>(t: DateTime<T>) {
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
    fn set<T: TimeZone>(t: DateTime<T>) {
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

#[derive(ValueEnum, Clone, Debug)]
enum Action {
    Set,
    Get,
}

#[derive(ValueEnum, Clone, Debug)]
enum Standart {
    Timestamp,
    RFC2822,
    RFC3339,
}

#[derive(Parser)]
#[command(name = "clock", about = "Time client", version = "0.1.1")]
struct App {
    #[arg(default_value = "get")]
    action: Action,
    #[arg(help = "When [ACTION] is set to 'set', apply [DATETIME]. Otherwise, ignore.")]
    datetime: Option<String>,
    #[arg(long = "use-standart", short = 's', default_value = "timestamp")]
    use_standart: Standart,
}

fn main() {
    let app = App::parse();
    match app.action {
        Action::Get => {
            let now = Clock::get();
            println!(
                "{}",
                match app.use_standart {
                    Standart::Timestamp => now.timestamp().to_string(),
                    Standart::RFC2822 => now.to_rfc2822(),
                    Standart::RFC3339 => now.to_rfc3339(),
                }
            );
        }
        Action::Set => {
            if let Some(input_datetime) = app.datetime {
                let err_msg = format!(
                    "Unable to parse '{}' according to {:?}",
                    input_datetime, app.use_standart
                );
                let datetime = match app.use_standart {
                    Standart::Timestamp => {
                        let ts: i64 = input_datetime.parse().expect("Datetime is not a number");
                        DateTime::from_timestamp(ts, 0)
                            .expect(&err_msg)
                            .fixed_offset()
                    }
                    Standart::RFC2822 => {
                        DateTime::parse_from_rfc2822(&input_datetime).expect(&err_msg)
                    }
                    Standart::RFC3339 => {
                        DateTime::parse_from_rfc3339(&input_datetime).expect(&err_msg)
                    }
                };
                Clock::set(datetime);
                
                let maybe_error = std::io::Error::last_os_error();
                let os_error_code = &maybe_error.raw_os_error();

                match os_error_code {
                    Some(0) => (),
                    Some(_) => eprintln!("Unalbe to set the time {:?}", maybe_error),
                    None => ()
                }
            }
        }
    }
}
