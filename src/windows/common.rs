
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

#[path="../events.rs"]
mod events;
use crate::events::events::{KeyboardEvent, MouseEvent, LinuxEvent};
use crate::events::events::*;

pub fn set_hook(
    hook_id: i32,
    hook_ptr: &AtomicPtr<HHOOK__>,
    hook_proc: unsafe extern "system" fn(c_int, WPARAM, LPARAM) -> LRESULT,
) {
    hook_ptr.store(
        unsafe { SetWindowsHookExW(hook_id, Some(hook_proc), 0 as HINSTANCE, 0) },
        Ordering::Relaxed,
    );
}

pub fn unset_hook(hook_ptr: &AtomicPtr<HHOOK__>) {
    if !hook_ptr.load(Ordering::Relaxed).is_null() {
        unsafe { UnhookWindowsHookEx(hook_ptr.load(Ordering::Relaxed)) };
        hook_ptr.store(null_mut(), Ordering::Relaxed);
    }
}
