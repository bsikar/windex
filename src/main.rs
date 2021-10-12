// good resource:
// https://tronche.com/gui/x/xlib/
mod client;
mod config;
mod windex;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use windex::Windex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // When a child processes ends, this process will not be signaled, it will be ignored.
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGCHLD, Arc::clone(&term))?;

    let mut wm = Windex::new();

    loop {
        wm.handle_events();

        std::thread::yield_now();
    }
}
