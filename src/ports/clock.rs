// The clock port. Time is an argument, never a reading: a journal entry that says
// when something happened is not reproducible if the writer asked the operating
// system, and a test that asserts on "now" is a test that fails at midnight.
//
// Unix seconds, signed, because a clock that can be wrong about the epoch is a
// fact of life and a `u64` would turn a pre-1970 timestamp into a huge number.

/// The current time, in whole seconds since the Unix epoch.
pub trait Clock {
    fn now_unix(&self) -> i64;
}
