use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Timestamp(i64);

impl Timestamp {
    pub fn from_unix_millis(ms: i64) -> Self {
        Self(ms)
    }

    pub fn as_unix_millis(&self) -> i64 {
        self.0
    }

    pub fn duration_ms(&self, other: &Timestamp) -> i64 {
        (self.0 - other.0).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_duration() {
        let t1 = Timestamp::from_unix_millis(1000);
        let t2 = Timestamp::from_unix_millis(5000);
        assert_eq!(t1.duration_ms(&t2), 4000);
        assert_eq!(t2.duration_ms(&t1), 4000);
        assert_eq!(t1.duration_ms(&t1), 0);
    }
}
