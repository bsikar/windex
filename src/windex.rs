use crate::client::*;
use crate::config::*;

use fork::{fork, Fork};
use std::mem::{drop, MaybeUninit};
use std::os::raw::c_int;
use std::process::Command;
use std::ptr::{null, null_mut};
use std::slice;
use x11::keysym::{XK_Num_Lock, XK_leftpointer};
use x11::xlib::*;

pub struct Windex<'a> {
    display: *mut Display,
    root_window: Window,
    display_height: i32,
    display_width: i32,
    window_width: u32,
    window_height: u32,
    window_x: i32,
    window_y: i32,
    workspace: u8,
    numlock: u32,
    event: *mut XEvent,
    mouse: *mut XButtonEvent,
    config: Config<'a>,
    client: *mut Client,
    cursor: *mut Client,
    workspaces: [*mut Client; 10],
}

impl<'a> Windex<'a> {
    pub fn new() -> Self {
        let display = unsafe { XOpenDisplay(null()) };

        if display.is_null() {
            panic!("cannot open display");
        }

        let default_screen = unsafe { XDefaultScreen(display) };
        let root_window = unsafe { XRootWindow(display, default_screen) };
        let display_width = unsafe { XDisplayWidth(display, default_screen) };
        let display_height = unsafe { XDisplayHeight(display, default_screen) };

        // XSetErrorHandler(xerror) xerror is a function that returns 0
        // XSetErrorHandler is more commonly used; it handles X11 protocol errors. It seems, Xlib
        // developers felt that any protocol error would be a bug and therefore the application should
        // exit with an error message like above.
        unsafe { XSetErrorHandler(Some(xerror)) };

        // The XSelectInput function requests that the X server report the events
        // associated with the specified event mask.
        unsafe { XSelectInput(display, root_window, SubstructureRedirectMask) };

        let cursor = unsafe { XCreateFontCursor(display, XK_leftpointer) };
        // If a cursor is set, it will be used when the pointer is in the window.
        unsafe { XDefineCursor(display, root_window, cursor) };

        let mut wm = Self {
            display,
            root_window,
            display_height,
            display_width,
            window_width: 0,
            window_height: 0,
            window_x: 0,
            window_y: 0,
            workspace: 1,
            numlock: 0,
            event: null_mut(),
            mouse: null_mut(),
            config: Config::new(),
            client: null_mut(),
            cursor: null_mut(),
            workspaces: [null_mut(); 10],
        };
        wm.input_grab();

        wm
    }

    fn input_grab(&mut self) {
        let modifiers = [0, LockMask, self.numlock, self.numlock | LockMask];
        let modmap = unsafe { XGetModifierMapping(self.display) };
        let max_keypermod = unsafe { (*modmap).max_keypermod } as usize;

        // KeyCode *modifiermap;   /* An 8 by max_keypermod array of the modifiers */
        let raw_data = unsafe { slice::from_raw_parts(modmap, 8 * max_keypermod) };

        for (i, data) in raw_data.iter().enumerate() {
            let modifiermap = data.modifiermap as u8;
            let keycode = unsafe { XKeysymToKeycode(self.display, XK_Num_Lock as u64) };

            if modifiermap == keycode {
                // TODO why does this work / why do we need it
                self.numlock = 1 << i;
            }
        }

        // The XUngrabKey function releases the key combination on the specified window
        // if it was grabbed by this client
        unsafe { XUngrabKey(self.display, AnyKey, AnyModifier, self.root_window) };

        for key in &self.config.keys {
            let keycode = unsafe { XKeysymToKeycode(self.display, key.keysym) } as i32;

            // If the specified KeySym is not defined for any KeyCode, XKeysymToKeycode
            // returns zero.
            if keycode != 0 {
                for modifier in modifiers {
                    unsafe {
                        XGrabKey(
                            self.display,
                            keycode,
                            key.modifier | modifier,
                            self.root_window,
                            True,
                            GrabModeAsync,
                            GrabModeAsync,
                        );
                    }
                }
            }
        }

        for i in (1..4).step_by(2) {
            for modifier in modifiers {
                unsafe {
                    XGrabButton(
                        self.display,
                        i,
                        MOD | modifier,
                        self.root_window,
                        True,
                        (ButtonPressMask | ButtonReleaseMask | PointerMotionMask) as u32,
                        GrabModeAsync,
                        GrabModeAsync,
                        0,
                        0,
                    );
                }
            }
        }

        unsafe { XFreeModifiermap(modmap) };
    }

    #[allow(non_upper_case_globals)]
    pub fn handle_events(&mut self) {
        // This fixes a bug:
        // https://stackoverflow.com/questions/68894089/xnextevent-in-rust-segfaults
        if self.event.is_null() {
            self.event = unsafe {
                let mut event = MaybeUninit::uninit();
                XNextEvent(self.display, event.as_mut_ptr());
                &mut event.assume_init()
            };
        }

        dbg!("handle");

        match unsafe { (*self.event).type_ } {
            ButtonPress => self.button_press(),
            KeyPress => self.key_press(),
            MapRequest => self.map_request(),
            _ => {}
        }
    }

    fn button_press(&mut self) {
        let subwindow = unsafe { (*self.event).button.subwindow };

        if subwindow > 0 {
            unsafe {
                XGetGeometry(
                    self.display,
                    subwindow,
                    null_mut(),
                    self.window_x as *mut i32,
                    self.window_y as *mut i32,
                    self.window_width as *mut u32,
                    self.window_height as *mut u32,
                    null_mut(),
                    null_mut(),
                );

                XRaiseWindow(self.display, subwindow);

                *self.mouse = (*self.event).button;
            }
        }
    }

    fn mod_clean(&self, mask: u32) -> u32 {
        mask & !(self.numlock | LockMask)
            & (ShiftMask | ControlMask | Mod1Mask | Mod2Mask | Mod3Mask | Mod4Mask | Mod5Mask)
    }

    fn run(&self, command: &'a str, args: &'a [&'a str]) {
        match fork() {
            Ok(Fork::Parent(_)) => {
                if unsafe { XInitThreads() } == 0 {
                    panic!("XInitThreads failed");
                }
            }
            Ok(Fork::Child) => {
                if !self.display.is_null() {
                    // NOTE this might break shit
                    unsafe { drop(XConnectionNumber(self.display)) };
                }
                Command::new(command)
                    .args(args)
                    .spawn()
                    .unwrap_or_else(|_| panic!("failed to execute {}", command));
            }
            Err(e) => panic!("{}", e),
        }
    }

    fn key_press(&self) {
        let keysym =
            unsafe { XkbKeycodeToKeysym(self.display, (*self.event).key.keycode as u8, 0, 0) };

        for key in &self.config.keys {
            if key.keysym == keysym
                && unsafe {
                    self.mod_clean((*self.event).key.state) == self.mod_clean(key.modifier)
                }
            {
                match key.function {
                    Functions::Run(c, a) => self.run(c, a),
                }
            }
        }
    }

    fn map_request(&mut self) {
        let window = unsafe { (*self.event).map_request.window };

        unsafe {
            XSelectInput(self.display, window, StructureNotifyMask | EnterWindowMask);

            XGetGeometry(
                self.display,
                window,
                null_mut(),
                self.window_x as *mut i32,
                self.window_y as *mut i32,
                self.window_width as *mut u32,
                self.window_height as *mut u32,
                null_mut(),
                null_mut(),
            );
            self.win_add(window);

            self.cursor = (*self.client).prev;

            if self.window_x + self.window_y == 0 {
                self.win_center();
            }

            XMapWindow(self.display, window);

            self.win_focus();
        }
    }

    fn win_center(&self) {
        if self.cursor.is_null() {
            return;
        }

        unsafe {
            XGetGeometry(
                self.display,
                (*self.cursor).window,
                null_mut(),
                null_mut(),
                null_mut(),
                self.window_width as *mut u32,
                self.window_height as *mut u32,
                null_mut(),
                null_mut(),
            );

            XMoveWindow(
                self.display,
                (*self.cursor).window,
                (self.display_width - self.window_width as i32) / 2,
                (self.display_height - self.window_height as i32) / 2,
            );
        }
    }

    fn win_focus(&mut self) {
        self.cursor = self.client;

        unsafe {
            XSetInputFocus(
                self.display,
                (*self.cursor).window,
                RevertToParent,
                CurrentTime,
            )
        };
    }

    fn win_add(&mut self, window: Window) {
        let mut c = Client::new();

        c.window = window;

        unsafe {
            if self.client.is_null() {
                self.client = &mut c;
                (*self.client).prev = &mut c;
                (*self.client).next = &mut c;
            } else {
                (*(*self.client).prev).next = &mut c;
                c.prev = (*self.client).prev;
                (*self.client).prev = &mut c;
                c.next = self.client;
            }
        }

        self.workspaces[self.workspace as usize] = self.client;
    }
}

extern "C" fn xerror(_display: *mut Display, _error: *mut XErrorEvent) -> c_int {
    0
}
