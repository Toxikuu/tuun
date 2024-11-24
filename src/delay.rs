use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

pub fn execute_after<F>(delay: Duration, func: F) -> Arc<Mutex<bool>>
where
    F: FnOnce() + Send + 'static
{
    let cancelled = Arc::new(Mutex::new(false));
    let cancelled_clone = cancelled.clone();

    thread::spawn(move || {
        let start = std::time::Instant::now();
        while start.elapsed() < delay {
            if *cancelled_clone.lock().unwrap() {
                println!("Cancelled task");
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        func();
    });

    cancelled
}
