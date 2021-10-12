use std::ptr::{null, null_mut};
use x11::xlib::{Window, XDefaultScreen, XDisplayHeight, XDisplayWidth, XOpenDisplay, XRootWindow};

#[derive(Copy, Clone)]
pub struct Client {
    pub next: *mut Client,
    pub prev: *mut Client,
    pub fullscreen: bool,
    pub workspace_x: i32,
    pub workspace_y: i32,
    pub workspace_width: i32,
    pub workspace_height: i32,
    pub window: Window,
}

impl Client {
    pub fn new() -> Self {
        let display = unsafe { XOpenDisplay(null()) };

        if display.is_null() {
            panic!("cannot open display");
        }

        let default_screen = unsafe { XDefaultScreen(display) };
        let window = unsafe { XRootWindow(display, default_screen) };
        let workspace_width = unsafe { XDisplayWidth(display, default_screen) };
        let workspace_height = unsafe { XDisplayHeight(display, default_screen) };

        Self {
            next: null_mut(),
            prev: null_mut(),
            fullscreen: false,
            workspace_x: 0,
            workspace_y: 0,
            workspace_width,
            workspace_height,
            window,
        }
    }
}
