use chrono::{DateTime, Local};
use clap::{Parser, ValueEnum};

struct Clock;

impl Clock {
    fn get() -> DateTime<Local> {
        Local::now()
    }

    fn set() -> ! {
        unimplemented!()
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

    #[arg(help = "When [ACTION] is setted to 'set', apply [DATETIME]. \
    Otherwise, ignore.")]
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
        Action::Set => unimplemented!(),
    }
}
