mod message;
use std::{net::UdpSocket, time::Duration};

use chrono::{DateTime, Utc};
pub use message::*;
mod result;
pub use result::*;
mod timestamp;
#[allow(unused_imports)]
pub use timestamp::*;

pub const NTP_MESSAGE_LENGTH: usize = 48;
pub const NTP_TO_UNIX_SECONDS: i64 = 2_208_988_800;
pub const LOCAL_ADDR: &'static str = "0.0.0.0:12300";

pub fn weighted_mean(values: &[f64], weights: &[f64]) -> f64 {
    let mut result = 0.0;
    let mut sum_of_weights = 0.0;

    for (v, w) in values.iter().zip(weights) {
        result += v * w;
        sum_of_weights += w;
    }

    result / sum_of_weights
}

pub fn ntp_roundtrip(host: &str, port: u16) -> Result<NTPResult, std::io::Error> {
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

pub fn check_time() -> Result<f64, std::io::Error> {
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
