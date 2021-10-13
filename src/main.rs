use fork::{fork, Fork};
use libc::close;
use std::mem::MaybeUninit;
use std::process::Command;
use std::ptr::null;
use x11::keysym::XK_Return;
use x11::xlib::*;

#[allow(non_upper_case_globals)]
fn main() {
    unsafe {
        let display = XOpenDisplay(null());

        if display.is_null() {
            panic!("cannot open display");
        }

        XGrabKey(
            display,
            XKeysymToKeycode(display, XK_Return as u64) as i32,
            Mod1Mask,
            XDefaultRootWindow(display),
            True,
            GrabModeAsync,
            GrabModeAsync,
        );

        loop {
            let event = {
                let mut event = MaybeUninit::uninit();
                XNextEvent(display, event.as_mut_ptr());
                &mut event.assume_init()
            };

            if event.type_ == KeyPress {
                match fork() {
                    Ok(Fork::Parent(_)) => {
                        if XInitThreads() == 0 {
                            panic!("XInitThreads failed");
                        }
                    }
                    Ok(Fork::Child) => {
                        if !display.is_null() {
                            // NOTE this might break shit
                            close(XConnectionNumber(display));
                        }

                        Command::new("st")
                            .spawn()
                            .unwrap_or_else(|_| panic!("failed to execute st"));
                    }
                    Err(e) => panic!("{}", e),
                }
            }
        }
    }
}
