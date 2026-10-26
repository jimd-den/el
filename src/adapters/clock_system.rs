// The system clock, and nothing else: one reading, at the edge, handed inward as a
// number (see `ports::clock` for why).

use crate::ports::clock::Clock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unix seconds from the operating system.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix(&self) -> i64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(since) => since.as_secs() as i64,
            // A clock set before 1970 is wrong, not impossible, and a journal entry
            // with a negative timestamp is still readable.
            Err(before) => -(before.duration().as_secs() as i64),
        }
    }
}
