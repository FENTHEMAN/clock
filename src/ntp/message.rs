use byteorder::{BigEndian, ReadBytesExt};

use super::{NTP_MESSAGE_LENGTH, timestamp::NTPTimestamp};

pub struct NTPMessage {
    pub data: [u8; NTP_MESSAGE_LENGTH],
}

impl NTPMessage {
    pub fn new() -> Self {
        NTPMessage {
            data: [0; NTP_MESSAGE_LENGTH],
        }
    }

    pub fn client() -> Self {
        const VERSION: u8 = 0b00_011_000;
        const MODE: u8 = 0b00_000_011;

        let mut msg = NTPMessage::new();

        msg.data[0] |= VERSION;
        msg.data[0] |= MODE;

        msg
    }

    pub fn parse_timestamp(&self, i: usize) -> Result<NTPTimestamp, std::io::Error> {
        let mut reader = &self.data[i..i + 8];
        let seconds = reader.read_u32::<BigEndian>()?;
        let fraction = reader.read_u32::<BigEndian>()?;

        Ok(NTPTimestamp { seconds, fraction })
    }

    pub fn rx_time(&self) -> Result<NTPTimestamp, std::io::Error> {
        self.parse_timestamp(32)
    }

    pub fn tx_time(&self) -> Result<NTPTimestamp, std::io::Error> {
        self.parse_timestamp(40)
    }
}
