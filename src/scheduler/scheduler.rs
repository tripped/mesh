extern crate time;

use self::time::Duration;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use std::thread;

struct Event<F> {
    time: u64,
    cb: F,
}

impl<F> Event<F> {
    fn new(time: u64, cb: F) -> Event<F> {
        Event {
            time: time,
            cb: cb
        }
    }

    fn fire(&self, actual: u64) {
        let drift = actual - self.time;
        println!("Event {} fired at {} => lag {}ns",
                 &self.time, actual, drift);
    }
}

// Events are ordered in reverse according to their scheduled time,
// hence we implement Ord and PartialOrd reversing the sense of cmp.
impl<F> Ord for Event<F> {
    fn cmp(&self, other: &Event<F>) -> Ordering {
        other.time.cmp(&self.time)
    }
}

impl<F> PartialOrd for Event<F> {
    fn partial_cmp(&self, other: &Event<F>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// We must also implement Eq, though this is strictly nonsense.
impl<F> Eq for Event<F> { }
impl<F> PartialEq for Event<F> {
    fn eq(&self, other: &Event<F>) -> bool {
        &self.time == &other.time
    }
}

#[test]
fn event_cmp() {
    // Because we order events with earliest time first, time=1 is
    // "bigger" than time=2. This is awkward, but made necessary by
    // the fact that std::collections::BinaryHeap is exclusively a
    // max-heap based on the contents' PartialOrd implementation.
    // This seems unhelpfully rigid (what if we want both a max-heap
    // AND a min-heap for the same type?), but c'est la vie.
    assert!(Event::new(1, ()) > Event::new(2, ()));
}

// A timer controls the scheduling of events based on the passage of time.
// Time here is a unitless 64-bit int, which it may be useful to interpret
// as milliseconds or nanoseconds.
struct Timer<F> {
    events: BinaryHeap<Event<F>>,
    elapsed: u64,
}

impl<F> Timer<F> {
    fn new() -> Timer<F> {
        Timer {
            events: BinaryHeap::new(),
            elapsed: 0
        }
    }

    // Schedule an event in the timer.
    fn add(&mut self, delay: u64, cb: F) {
        self.events.push(Event::new(delay + self.elapsed, cb));
    }

    // Get the time remaining to the earliest pending event,
    // if there is one; None otherwise.
    fn earliest(&self) -> Option<u64> {
        self.events.peek().map(|e| e.time - self.elapsed)
    }

    // Advance time by a specified duration, firing all scheduled
    // events whose timeout period has now elapsed.
    // Return the time remaining to the next event, if any.
    fn advance(&mut self, elapsed: u64) -> Option<u64> {
        self.elapsed += elapsed;
        while self.events.peek().map_or(false, |e| e.time <= self.elapsed) {
            self.events.pop().unwrap().fire(self.elapsed);
        }
        self.earliest()
    }
}

#[test]
fn timer_earliest_no_events() {
    let t = Timer::<()>::new();
    assert_eq!(t.earliest(), None);
}

#[test]
fn timer_earliest_one() {
    let mut t = Timer::new();
    t.add(100, ());
    assert_eq!(t.earliest(), Some(100));
}

#[test]
fn timer_earliest_orders_correctly() {
    let mut t = Timer::new();
    t.add(100, ());
    t.add(10, ());
    t.add(50, ());
    assert_eq!(t.earliest(), Some(10));
}

#[test]
fn timer_earliest_updates_after_advance() {
    let mut t = Timer::new();
    t.add(100, ());
    t.advance(58);
    assert_eq!(t.earliest(), Some(42));
}

#[test]
fn timer_advance_pops_events() {
    let mut t = Timer::new();
    t.add(2, ());
    t.add(3, ());
    t.add(1, ());
    for n in 1..4 {
        // NB: delta to next earliest is 1 each time
        assert_eq!(t.earliest(), Some(1));
        t.advance(1);
    }
    assert_eq!(t.earliest(), None);
}

#[test]
fn timer_advance_pops_multiple() {
    let mut t = Timer::new();
    t.add(1, ());
    t.add(2, ());
    t.add(3, ());
    t.add(10, ());
    t.add(10, ());
    t.add(14, ());
    t.advance(10);
    assert_eq!(t.earliest(), Some(4));
}

#[test]
fn timer_add_after_advance() {
    let mut t = Timer::new();
    t.advance(1000);
    t.add(1, ());
    assert_eq!(t.earliest(), Some(1));
}

pub struct Scheduler {
    timer: Arc<Mutex<Timer<()>>>,
    timer_thread: thread::JoinHandle<()>,
}

impl Scheduler {
    fn new() -> Scheduler {
        let timer: Arc<Mutex<Timer<()>>>
            = Arc::new(Mutex::new(Timer::new()));

        let timer_thread = {
            let timer = timer.clone();
            thread::spawn(move || {
                // How long we plan to park the thread, in nanoseconds.
                // None means "park until somebody schedules an event."
                let mut wait = None;

                loop {
                    // Measure how long we actually spend parked
                    let elapsed = Duration::span(|| {
                        if let Some(ns) = wait {
                            thread::park_timeout_ms((ns/1000000) as u32);
                        } else {
                            thread::park();
                        }
                    }).num_nanoseconds().unwrap() as u64;

                    // Advance the timer and decide how long to wait again
                    let mut timer = timer.lock().unwrap();
                    wait = timer.advance(elapsed);
                }
            })
        };

        Scheduler { 
            timer: timer,
            timer_thread: timer_thread,
        }
    }

    // Schedule the execution of a nullary closure returning unit
    // after a specified time period in milliseconds.
    fn delay(&mut self, millis: u64, func: ()) {
        let mut timer = self.timer.lock().unwrap();
        timer.add(millis * 1000000, func);
        self.timer_thread.thread().unpark();
    }

    // Run the scheduler loop forever. FOREEEEVER.
    fn run(self) {
        self.timer_thread.join();
    }
}

#[test]
fn crappy_threaded_scheduler_test() {
    let mut s = Scheduler::new();
    s.delay(949, ());
    s.delay(1001, ());
    s.delay(500, ());
    s.delay(200, ());
    s.run();
}
