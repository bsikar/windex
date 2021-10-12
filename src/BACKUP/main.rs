// good resource:
// https://tronche.com/gui/x/xlib/
mod client;
mod config;
mod windex;

use client::*;
use config::*;
use std::cmp::max;
use std::os::raw::c_int;
use std::ptr::{null, null_mut};
use std::slice;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use windex::*;
use x11::keysym::{XK_Num_Lock, XK_leftpointer};
use x11::xlib::{
    AnyKey, AnyModifier, ButtonPress, ButtonPressMask, ButtonRelease, ButtonReleaseMask,
    ConfigureRequest, ControlMask, CurrentTime, DestroyNotify, Display, EnterNotify,
    EnterWindowMask, GrabModeAsync, KeyPress, LockMask, MapRequest, MappingKeyboard,
    MappingModifier, MappingNotify, Mod1Mask, Mod2Mask, Mod3Mask, Mod4Mask, Mod5Mask, MotionNotify,
    PointerMotionMask, RevertToParent, ShiftMask, StructureNotifyMask, SubstructureRedirectMask,
    True, Window, XButtonEvent, XCheckTypedEvent, XConfigureWindow, XCreateFontCursor,
    XDefaultScreen, XDefineCursor, XDisplayHeight, XDisplayWidth, XErrorEvent, XEvent,
    XFreeModifiermap, XGetGeometry, XGetModifierMapping, XGrabButton, XGrabKey, XKeysymToKeycode,
    XMoveResizeWindow, XNextEvent, XOpenDisplay, XRaiseWindow, XRefreshKeyboardMapping,
    XRootWindow, XSelectInput, XSetErrorHandler, XSetInputFocus, XUngrabKey, XWindowChanges,
    XkbKeycodeToKeysym,
};

struct Windex<'a> {
    display: *mut Display,
    root_window: Window,
    display_height: i32,
    display_width: i32,
    workspace_width: u32,
    workspace_height: u32,
    workspace_x: i32,
    workspace_y: i32,
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
    fn new() -> Self {
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
            workspace_width: 0,
            workspace_height: 0,
            workspace_x: 0,
            workspace_y: 0,
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

        for key in self.config.keys {
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
    fn handle_events(&mut self) {
        match unsafe { (*self.event).type_ } {
            ButtonPress => self.button_press(),
            ButtonRelease => self.button_release(),
            ConfigureRequest => self.configure_request(),
            KeyPress => self.key_press(),
            MapRequest => self.map_request(),
            MappingNotify => self.mapping_notify(),
            DestroyNotify => self.notify_destroy(),
            EnterNotify => self.notify_enter(),
            MotionNotify => unsafe { self.notify_motion() },
            _ => {}
        }
    }

    fn button_press(&mut self) {
        let subwindow = unsafe { (*self.event).button.subwindow };

        if subwindow != 0 {
            unsafe {
                XGetGeometry(
                    self.display,
                    subwindow,
                    0 as *mut u64,
                    self.workspace_x as *mut i32,
                    self.workspace_y as *mut i32,
                    self.workspace_width as *mut u32,
                    self.workspace_height as *mut u32,
                    0 as *mut u32,
                    0 as *mut u32,
                );

                XRaiseWindow(self.display, subwindow);

                *self.mouse = (*self.event).button;
            }
        }
    }

    fn button_release(&mut self) {
        unsafe { (*self.mouse).subwindow = 0 };
    }

    fn configure_request(&self) {
        let ev = unsafe { (*self.event).configure_request };

        let mut changes = XWindowChanges {
            x: ev.x,
            y: ev.y,
            width: ev.width,
            height: ev.height,
            sibling: ev.above,
            stack_mode: ev.detail,
            border_width: ev.border_width,
        };

        unsafe { XConfigureWindow(self.display, ev.window, ev.value_mask as u32, &mut changes) };
    }

    fn mod_clean(&self, mask: u32) -> u32 {
        mask & !(self.numlock | LockMask)
            & (ShiftMask | ControlMask | Mod1Mask | Mod2Mask | Mod3Mask | Mod4Mask | Mod5Mask)
    }

    fn key_press(&self) {
        let keysym =
            unsafe { XkbKeycodeToKeysym(self.display, (*self.event).key.keycode as u8, 0, 0) };

        for key in self.config.keys {
            if key.keysym == keysym
                && unsafe {
                    self.mod_clean((*self.event).key.state) == self.mod_clean(key.modifier)
                }
            {
                (key.function)(key.arg);
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
                0 as *mut u64,
                self.workspace_x as *mut i32,
                self.workspace_y as *mut i32,
                self.workspace_width as *mut u32,
                self.workspace_height as *mut u32,
                0 as *mut u32,
                0 as *mut u32,
            );

            self.win_add(window);
        }
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

    fn mapping_notify(&mut self) {
        let mut event = unsafe { (*self.event).mapping };

        if event.request == MappingKeyboard || event.request == MappingModifier {
            unsafe { XRefreshKeyboardMapping(&mut event) };
            self.input_grab();
        }
    }

    fn window_delete(&mut self, window: Window) {
        let mut x: *mut Client = null_mut();
        let mut t: *mut Client = null_mut();
        let mut c = self.client;

        while !c.is_null() && unsafe { t != (*self.client).prev } {
            if unsafe { (*c).window == window } {
                x = c;
            }

            t = c;
            c = unsafe { (*c).next };
        }

        if self.client.is_null() || x.is_null() {
            return;
        }

        if unsafe { (*x).prev == x } {
            self.client = null_mut();
        }

        if self.client == x {
            self.client = unsafe { (*x).next };
        }

        if unsafe { !(*x).next.is_null() } {
            unsafe { (*(*x).next).prev = (*x).prev };
        }

        if unsafe { !(*x).prev.is_null() } {
            unsafe { (*(*x).prev).next = (*x).next };
        }

        self.workspaces[self.workspace as usize] = self.client;
    }

    fn notify_destroy(&mut self) {
        unsafe { self.window_delete((*self.event).destroy_window.window) };

        if !self.client.is_null() {
            unsafe {
                self.cursor = (*self.client).prev;

                XSetInputFocus(
                    self.display,
                    (*self.cursor).window,
                    RevertToParent,
                    CurrentTime,
                );
            }
        }
    }

    fn notify_enter(&mut self) {
        while unsafe { XCheckTypedEvent(self.display, EnterNotify, self.event) != 0 } {
            std::thread::yield_now();
        }

        let mut t: *mut Client = null_mut();
        let mut c = self.client;

        while !c.is_null() && unsafe { t != (*self.client).prev } {
            if unsafe { (*c).window == (*self.event).crossing.window } {
                self.cursor = c;

                unsafe {
                    XSetInputFocus(
                        self.display,
                        (*self.cursor).window,
                        RevertToParent,
                        CurrentTime,
                    )
                };
            }

            t = c;
            c = unsafe { (*c).next };
        }
    }

    unsafe fn notify_motion(&mut self) {
        if (*self.mouse).subwindow == 0 || (*self.cursor).fullscreen {
            return;
        }

        while XCheckTypedEvent(self.display, MotionNotify, self.event) != 0 {
            std::thread::yield_now();
        }

        let dx = (*self.event).button.x_root - (*self.mouse).x_root;
        let dy = (*self.event).button.y_root - (*self.mouse).y_root;

        let x = self.workspace_x + if (*self.mouse).button == 1 { dx } else { 0 };
        let y = self.workspace_y + if (*self.mouse).button == 1 { dy } else { 0 };
        let width = max(
            1,
            self.workspace_width + if (*self.mouse).button == 3 { dx } else { 0 } as u32,
        );
        let height = max(
            1,
            self.workspace_height + if (*self.mouse).button == 3 { dy } else { 0 } as u32,
        );

        XMoveResizeWindow(self.display, (*self.mouse).subwindow, x, y, width, height);
    }
}

extern "C" fn xerror(_display: *mut Display, _error: *mut XErrorEvent) -> c_int {
    0
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // When a child processes ends, this process will not be signaled, it will be ignored.
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGCHLD, Arc::clone(&term))?;

    let mut wm = Windex::new();

    loop {
        unsafe {
            XNextEvent(wm.display, wm.event);

            wm.handle_events();
        }

        std::thread::yield_now();
    }
}
