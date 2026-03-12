use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct NTPResult {
    pub t1: DateTime<Utc>,
    pub t2: DateTime<Utc>,
    pub t3: DateTime<Utc>,
    pub t4: DateTime<Utc>,
}

impl NTPResult {
    pub fn offset(&self) -> i64 {
        let offset = (self.t2 - self.t1) + (self.t3 - self.t4);
        offset.num_milliseconds() / 2
    }

    pub fn delay(&self) -> i64 {
        let duration = (self.t4 - self.t1) - (self.t3 - self.t2);
        duration.num_milliseconds()
    }
}
