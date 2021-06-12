use std::os::raw::{c_int, c_long, c_ulong};
use std::ptr::{null as NULL, null_mut as NULL_MUT};
use x11::xlib::*;
use x11::xlib_xcb::*;

struct WindowManager {
    dpy: *mut Display,
    xerrorstart: Option<unsafe extern "C" fn(*mut Display, *mut XErrorEvent) -> c_int>,
    xerrorxlib: Option<unsafe extern "C" fn(*mut Display, *mut XErrorEvent) -> c_int>,
    xerror: Option<unsafe extern "C" fn(*mut Display, *mut XErrorEvent) -> c_int>,
    xcon: *mut xcb_connection_t,
}

impl WindowManager {
    unsafe fn check_other_wm(&mut self) {
        self.xerrorxlib = XSetErrorHandler(self.xerrorstart);

        // this causes an error if another wm is running
        XSelectInput(
            self.dpy,
            XDefaultRootWindow(self.dpy),
            SubstructureRedirectMask,
        );
        XSync(self.dpy, 0);
        XSetErrorHandler(self.xerror);
        XSync(self.dpy, 0);
    }
}

fn main() {
    // CLAP cli parameter stuff here

    let dpy = unsafe { XOpenDisplay(NULL()) };
    let mut wm = WindowManager {
        dpy,
        xerrorstart: None,
        xerrorxlib: None,
        xerror: None,
        xcon: unsafe { XGetXCBConnection(dpy) },
    };

    if wm.dpy == NULL_MUT() {
        panic!("cannot open display");
    }

    if wm.xcon == NULL_MUT() {
        panic!("cannot get xcb connection");
    }

    unsafe { wm.check_other_wm() };

    loop { std::thread::yield_now() };

    // XrmInitialize()
    // loadxrdb()
    // setup()
    // scan()
    // runAutostart()
    // run()
    //
    // RESTART:
    //  execvp(); // (argv[0], argv)
    // cleanup()
    // close x
}
