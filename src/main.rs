mod clock;
mod ntp;

use clock::Clock;
use ntp::check_time;

use chrono::DateTime;
use chrono::Utc;
use clap::{Parser, ValueEnum};

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

fn catch_os_err() {
    let maybe_error = std::io::Error::last_os_error();
    let os_error_code = &maybe_error.raw_os_error();

    match os_error_code {
        Some(0) => (),
        Some(_) => eprintln!("Unalbe to set the time {:?}", maybe_error),
        None => (),
    }
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
                catch_os_err();
            }
        }
        Action::NTP => {
            let offset = check_time().unwrap() as isize;

            let adjust_ms_ = offset.signum() * offset.abs().min(200) / 5;
            let adjust_ms = chrono::Duration::milliseconds(adjust_ms_ as i64);

            let now: DateTime<Utc> = Utc::now() + adjust_ms;

            Clock::set(now);
            catch_os_err();
        }
    }
}
