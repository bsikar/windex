use std::os::raw::c_uint;
use x11::xlib::{KeySym, Mod1Mask, Window};

pub const MOD: c_uint = Mod1Mask;

#[derive(Copy, Clone)]
pub struct Arg<'a> {
    pub command: &'a [&'a str],
    pub workspace: u8,
    pub window: Window,
}

impl<'a> Arg<'a> {
    fn new(command: &'a [&'a str], workspace: u8, window: Window) -> Self {
        Self {
            command,
            workspace,
            window,
        }
    }
}

pub struct Key<'a> {
    pub modifier: u32,
    pub keysym: KeySym,
    pub function: fn(Arg),
    pub arg: Option<Arg<'a>>,
}

impl<'a> Key<'a> {
    fn new(modifier: u32, keysym: KeySym, function: fn(Arg), arg: Option<Arg<'a>>) -> Self {
        Self {
            modifier,
            keysym,
            function,
            arg,
        }
    }
}

pub struct Config<'a> {
    pub keys: &'a [Key<'a>],
}

impl<'a> Config<'a> {
    pub fn new() -> Self {
        Self {
            keys: &[Key::new(MOD, XK_q, win_kill, None)],
        }
    }
}
