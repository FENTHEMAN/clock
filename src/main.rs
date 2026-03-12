use std::net::UdpSocket;
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};
use chrono::{DateTime, Local};
use chrono::{TimeZone, Timelike, Utc};
use clap::{Parser, ValueEnum};

const NTP_MESSAGE_LENGTH: usize = 48;
const NTP_TO_UNIX_SECONDS: i64 = 2_208_988_800;
const LOCAL_ADDR: &'static str = "0.0.0.0:12300";

#[derive(Default, Debug, Copy, Clone)]
struct NTPTimestamp {
    seconds: u32,
    fraction: u32,
}

struct NTPMessage {
    data: [u8; NTP_MESSAGE_LENGTH],
}

#[derive(Debug)]
struct NTPResult {
    t1: DateTime<Utc>,
    t2: DateTime<Utc>,
    t3: DateTime<Utc>,
    t4: DateTime<Utc>,
}

impl NTPResult {
    fn offset(&self) -> i64 {
        let offset = (self.t2 - self.t1) + (self.t3 - self.t4);
        offset.num_milliseconds() / 2
    }

    fn delay(&self) -> i64 {
        let duration = (self.t4 - self.t1) - (self.t3 - self.t2);
        duration.num_milliseconds()
    }
}

impl From<NTPTimestamp> for DateTime<Utc> {
    fn from(ntp: NTPTimestamp) -> Self {
        let secs = ntp.seconds as i64 - NTP_TO_UNIX_SECONDS;
        let mut nanos = ntp.fraction as f64;
        nanos *= 1e9;
        nanos /= 2_f64.powi(32);

        Utc.timestamp_opt(secs, nanos as u32).unwrap()
    }
}

impl From<DateTime<Utc>> for NTPTimestamp {
    fn from(utc: DateTime<Utc>) -> Self {
        let secs = utc.timestamp() + NTP_TO_UNIX_SECONDS;
        let mut fraction = utc.nanosecond() as f64;
        fraction *= 2_f64.powi(32);
        fraction /= 1e9;

        NTPTimestamp {
            seconds: secs as u32,
            fraction: fraction as u32,
        }
    }
}

impl NTPMessage {
    fn new() -> Self {
        NTPMessage {
            data: [0; NTP_MESSAGE_LENGTH],
        }
    }

    fn client() -> Self {
        const VERSION: u8 = 0b00_011_000;
        const MODE: u8 = 0b00_000_011;

        let mut msg = NTPMessage::new();

        msg.data[0] |= VERSION;
        msg.data[0] |= MODE;

        msg
    }

    fn parse_timestamp(&self, i: usize) -> Result<NTPTimestamp, std::io::Error> {
        let mut reader = &self.data[i..i + 8];
        let seconds = reader.read_u32::<BigEndian>()?;
        let fraction = reader.read_u32::<BigEndian>()?;

        Ok(NTPTimestamp { seconds, fraction })
    }

    fn rx_time(&self) -> Result<NTPTimestamp, std::io::Error> {
        self.parse_timestamp(32)
    }

    fn tx_time(&self) -> Result<NTPTimestamp, std::io::Error> {
        self.parse_timestamp(40)
    }
}

fn weighted_mean(values: &[f64], weights: &[f64]) -> f64 {
    let mut result = 0.0;
    let mut sum_of_weights = 0.0;

    for (v, w) in values.iter().zip(weights) {
        result += v * w;
        sum_of_weights += w;
    }

    result / sum_of_weights
}

fn ntp_roundtrip(host: &str, port: u16) -> Result<NTPResult, std::io::Error> {
    let destination = format!("{}:{}", host, port);

    let request = NTPMessage::client();
    let mut response = NTPMessage::new();

    let message = request.data;

    let udp = UdpSocket::bind(LOCAL_ADDR)?;
    udp.connect(&destination).expect("Unable to connect");

    let t1 = Utc::now();

    udp.send(&message)?;
    udp.set_read_timeout(Some(Duration::from_secs(1)))?;
    udp.recv_from(&mut response.data)?;
    let t4 = Utc::now();

    let t2: DateTime<Utc> = response.rx_time().unwrap().into();
    let t3: DateTime<Utc> = response.tx_time().unwrap().into();

    Ok(NTPResult { t1, t2, t3, t4 })
}

fn check_time() -> Result<f64, std::io::Error> {
    const NTP_PORT: u16 = 123;

    let servers = [
        "time.nist.gov",
        "time.apple.com",
        "time.euro.apple.com",
        "time.google.com",
        "time2.google.com",
    ];

    let mut times = Vec::with_capacity(servers.len());

    for &server in servers.iter() {
        print!("{} =>", server);

        let calc = ntp_roundtrip(&server, NTP_PORT);

        match calc {
            Ok(time) => {
                println!(" {}ms away from local system time", time.offset());
                times.push(time);
            }
            Err(_) => {
                println!(" ? [response took too long]")
            }
        };
    }

    let mut offsets = Vec::with_capacity(servers.len());
    let mut offset_weights = Vec::with_capacity(servers.len());

    for time in &times {
        let offset = time.offset() as f64;
        let delay = time.delay() as f64;

        let weight = 1_000_000.0 / (delay * delay);
        if weight.is_finite() {
            offsets.push(offset);
            offset_weights.push(weight);
        }
    }

    let avg_offset = weighted_mean(&offsets, &offset_weights);

    Ok(avg_offset)
}

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
    NTP,
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
            }
        }
        Action::NTP => {
            let offset = check_time().unwrap() as isize;

            let adjust_ms_ = offset.signum() * offset.abs().min(200) / 5;
            let adjust_ms = chrono::Duration::milliseconds(adjust_ms_ as i64);

            let now: DateTime<Utc> = Utc::now() + adjust_ms;

            Clock::set(now);
        }
    }
    let maybe_error = std::io::Error::last_os_error();
    let os_error_code = &maybe_error.raw_os_error();

    match os_error_code {
        Some(0) => (),
        Some(_) => eprintln!("Unalbe to set the time {:?}", maybe_error),
        None => (),
    }
}
