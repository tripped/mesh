extern crate time;

use self::time::{Duration, SteadyTime};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::thread;
use std::sync::{Arc, Mutex};

struct Event {
    time: SteadyTime,
    //cb: Arc<Mutex<Fn() -> ()>>,
}

impl Event {
    fn expired(&self) -> bool {
        SteadyTime::now() > self.time
    }

    fn fire(&self) {
        let now = SteadyTime::now();
        let drift = now - self.time;
        println!("Event {} fired at {} => drift {}", &self.time, now, drift);
    }
}

//
// Events are ordered in reverse according to their scheduled time,
// hence we implement Ord and PartialOrd reversing the sense of cmp.
//
impl Ord for Event {
    fn cmp(&self, other: &Event) -> Ordering {
        other.time.cmp(&self.time)
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Event) -> Option<Ordering> {
        Some(other.time.cmp(&self.time))
    }
}

// We must also implement Eq, though this is strictly nonsense.
impl Eq for Event { }
impl PartialEq for Event {
    fn eq(&self, other: &Event) -> bool {
        &self.time == &other.time
    }
}

pub struct Scheduler {
    events: Arc<Mutex<BinaryHeap<Event>>>,
    timer_thread: thread::JoinHandle<()>,
}

impl Scheduler {
    fn new() -> Scheduler {
        let queue: Arc<Mutex<BinaryHeap<Event>>>
            = Arc::new(Mutex::new(BinaryHeap::new()));

        // The timer thread loops over a priority queue of upcoming timer
        // events; firing any that have passed their expiry time. When no
        // events are expired, it parks until the next one is scheduled to
        // fire, or forever if there are no events in the queue at all.
        // The thread is unparked by the insertion of a new event into the
        // queue, at which point it resumes looping.
        let q = queue.clone();
        let timer_thread = thread::spawn(move || { loop {
            let mut wait = None;

            {
                let mut queue = q.lock().unwrap();

                // Pump expired events
                loop {
                    let fire = match queue.peek() {
                        Some(event) if event.expired() => true,
                        _ => false
                    };

                    if fire {
                        queue.pop().unwrap().fire();
                    } else {
                        break;
                    }
                }

                // If there's an upcoming event, figure out how long to sleep
                if let Some(event) = queue.peek() {
                    wait = Some(event.time - SteadyTime::now());
                }
            }

            // Park the thread
            match wait {
                Some(d) => thread::park_timeout_ms(d.num_milliseconds() as u32),
                _ => thread::park()
            }
        }});

        Scheduler { 
            events: queue,
            timer_thread: timer_thread,
        }
    }

    // Schedule the execution of a nullary closure returning unit
    // after a specified time period in milliseconds.
    fn delay<F>(&mut self, millis: u32, func: F)
        where F: Fn() -> () {
        let mut events = self.events.lock().unwrap();
        events.push(Event {
            time: SteadyTime::now() + Duration::milliseconds(millis as i64)
        });

        self.timer_thread.thread().unpark();
    }

    // Run the scheduler loop forever. FOREEEEVER.
    fn run(self) {
        self.timer_thread.join();
    }
}

#[test]
fn crappy_threaded_scheduler_test() {
    // TODO: the logic of scheduling should be separated from the
    // threaded driver run by the Scheduler so it can be unit tested.
    let mut s = Scheduler::new();
    s.delay(1000, || {});
    s.delay(1001, || {});
    s.delay(500, || {});
    s.delay(200, || {});
    s.run();
}
