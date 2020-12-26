
pub use crossbeam_channel::{unbounded,Sender, Receiver};
pub use lazy_static::lazy_static;
pub use std::sync::Mutex;
pub use std::thread;
pub extern crate user32;
pub extern crate winapi;
pub use std::{
    mem::{size_of, transmute_copy, MaybeUninit},
    ptr::null_mut,
    sync::atomic::{AtomicPtr, Ordering},
};
pub use winapi::{
    ctypes::*,
    shared::{minwindef::*, windef::*},
    um::winuser::*,
};
pub use std::process;
pub use once_cell::sync::Lazy;


mod common;
pub use common::*;
mod keyboard;
pub use keyboard::*;
mod mouse;
pub use mouse::*;
