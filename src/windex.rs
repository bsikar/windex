use crate::config::*;
use fork::{fork, Fork};
use libc::close;
use std::os::raw::c_int;
use std::ptr::{null, null_mut};
use std::{mem::MaybeUninit, process::Command};
use x11::keysym::XK_leftpointer;
use x11::xlib::*;

pub struct Windex<'a> {
    display: *mut Display,
    event: *mut XEvent,
    config: Config<'a>,
}

impl<'a> Windex<'a> {
    pub fn new() -> Self {
        let display = unsafe { XOpenDisplay(null()) };
        if display.is_null() {
            panic!("cannot open display");
        }

        let root_window = unsafe { XDefaultRootWindow(display) };

        unsafe { XSetErrorHandler(Some(xerror)) };

        let cursor = unsafe { XCreateFontCursor(display, XK_leftpointer) };
        unsafe { XDefineCursor(display, root_window, cursor) };

        Self {
            display,
            event: null_mut(),
            config: Config::new(),
        }
    }

    #[allow(non_upper_case_globals)]
    pub fn run(&mut self) {
        for key in self.config.keys.clone() {
            unsafe {
                XGrabKey(
                    self.display,
                    XKeysymToKeycode(self.display, key.keysym) as i32,
                    key.modifier,
                    XDefaultRootWindow(self.display),
                    True,
                    GrabModeAsync,
                    GrabModeAsync,
                )
            };
        }

        loop {
            self.event = {
                let mut event = MaybeUninit::uninit();
                unsafe { XNextEvent(self.display, event.as_mut_ptr()) };
                unsafe { &mut event.assume_init() }
            };

            if unsafe { (*self.event).type_ } == KeyPress {
                self.key_press()
            }
        }
    }

    fn key_press(&mut self) {
        let keysym =
            unsafe { XkbKeycodeToKeysym(self.display, (*self.event).key.keycode as u8, 0, 0) };

        for key in self.config.keys.clone() {
            if key.keysym == keysym {
                match key.function {
                    Functions::WindowKill => self.window_kill(),
                    Functions::Run(c, a) => self.run_command(c, a),
                }
            }
        }
    }

    fn window_kill(&mut self) {
        // TODO: remove the window
    }

    fn run_command(&mut self, command: &'a str, args: &'a [&'a str]) {
        match fork() {
            Ok(Fork::Parent(_)) => {
                if unsafe { XInitThreads() == 0 } {
                    panic!("XInitThreads failed");
                }
            }
            Ok(Fork::Child) => {
                if !self.display.is_null() {
                    // XXX: this might break shit
                    unsafe { close(XConnectionNumber(self.display)) };
                }

                Command::new(command)
                    .args(args)
                    .spawn()
                    .unwrap_or_else(|_| panic!("failed to execute {}", command));
            }
            Err(e) => panic!("{}", e),
        }
    }
}

extern "C" fn xerror(_display: *mut Display, _error: *mut XErrorEvent) -> c_int {
    0
}
