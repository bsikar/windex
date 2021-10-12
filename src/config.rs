use std::os::raw::c_uint;
use x11::keysym::*;
use x11::xlib::{KeySym, Mod1Mask};

pub const MOD: c_uint = Mod1Mask;

#[derive(Copy, Clone)]
pub enum Functions<'a> {
    WindowKill,
    WindowCenter,
    WindowFullScreen,
    Run(&'a str),
}

pub struct Key<'a> {
    pub modifier: u32,
    pub keysym: KeySym,
    pub function: Functions<'a>,
}

impl<'a> Key<'a> {
    fn new(modifier: u32, keysym: KeySym, function: Functions<'a>) -> Self {
        Self {
            modifier,
            keysym,
            function,
        }
    }
}

pub struct Config<'a> {
    pub keys: Vec<Key<'a>>,
}

impl<'a> Config<'a> {
    pub fn new() -> Self {
        use Functions::*;

        let keys = vec![
            Key::new(MOD, XK_q as u64, WindowKill),
            Key::new(MOD, XK_Return as u64, Run("st")),
        ];

        Self { keys }
    }
}
