extern crate time;

use self::time::{Duration, PreciseTime};

struct Timer {
    time: PreciseTime,
    cb: FnOnce() -> (),
}

pub struct Scheduler<'a> {
    timers: Vec<&'a Timer>
}

impl<'a> Scheduler<'a> {
    // Schedule the execution of a nullary closure returning unit
    // after a specified time period in milliseconds.
    fn delay<F: FnOnce() -> ()>(self, millis: u32, func: F) {
    }

    // Run the scheduler loop forever. FOREEEEVER.
    fn run(self) {
    }
}


