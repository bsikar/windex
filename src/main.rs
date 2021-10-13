use fork::{fork, Fork};
use libc::close;
use std::{mem::MaybeUninit, process::Command, ptr::null};
use x11::keysym::XK_Return;
use x11::xlib::{
    GrabModeAsync, KeyPress, Mod1Mask, True, XConnectionNumber, XDefaultRootWindow, XGrabKey,
    XInitThreads, XKeysymToKeycode, XNextEvent, XOpenDisplay,
};

#[allow(non_upper_case_globals)]
fn main() {
    let display = unsafe { XOpenDisplay(null()) };

    if display.is_null() {
        panic!("cannot open display");
    }

    unsafe {
        XGrabKey(
            display,
            XKeysymToKeycode(display, XK_Return as u64) as i32,
            Mod1Mask,
            XDefaultRootWindow(display),
            True,
            GrabModeAsync,
            GrabModeAsync,
        )
    };

    loop {
        let event = {
            let mut event = MaybeUninit::uninit();
            unsafe { XNextEvent(display, event.as_mut_ptr()) };
            unsafe { &mut event.assume_init() }
        };

        if unsafe { event.type_ == KeyPress } {
            match fork() {
                Ok(Fork::Parent(_)) => {
                    if unsafe { XInitThreads() == 0 } {
                        panic!("XInitThreads failed");
                    }
                }
                Ok(Fork::Child) => {
                    if !display.is_null() {
                        // XXX: this might break shit
                        unsafe { close(XConnectionNumber(display)) };
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
