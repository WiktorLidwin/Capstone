use input::{
    event::{
        keyboard::{
            KeyState, {KeyboardEvent, KeyboardEventTrait},
        },
        pointer::{ButtonState, PointerEvent::*},
        Event::{self, *},
        EventTrait,
    },
    Libinput, LibinputInterface,
    Device,
    SendEventsMode,
};
use nix::{
    fcntl::{open, OFlag},
    sys::stat::Mode,
    unistd::close,
};
use std::{
    os::unix::io::RawFd, path::Path, thread::sleep, time::Duration, ptr::null, mem::MaybeUninit,
};
pub use std::{
    collections::hash_map::HashMap,
    sync::atomic::{AtomicPtr, Ordering},
    sync::{Arc, Mutex},
    thread::spawn,
};
use uinput::event::relative::Position;
use x11::{xlib::*, xtest::*};
use once_cell::sync::Lazy;

struct LibinputInterfaceRaw;

impl LibinputInterfaceRaw {
    fn seat(&self) -> String {
        String::from("seat0")
    }
}

impl LibinputInterface for LibinputInterfaceRaw {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> std::result::Result<RawFd, i32> {
        println!("path {:?} ", path);
        if let Ok(fd) = open(path, OFlag::from_bits_truncate(flags), Mode::all()) {
            Ok(fd)
        } else {
            Err(1)
        }
    }

    fn close_restricted(&mut self, fd: RawFd) {
        let _ = close(fd);
    }
}

pub fn handle_input_events() {
    let udev_context = udev::Context::new().unwrap();
    let mut libinput_context = Libinput::new_from_udev(LibinputInterfaceRaw, &udev_context);
    libinput_context
        .udev_assign_seat(&LibinputInterfaceRaw.seat())
        .unwrap();
    while true {
        libinput_context.dispatch().unwrap();
        while let Some(event) = libinput_context.next() {
            handle_input_event(event);
            
        }
        sleep(Duration::from_millis(10));
    }
}

fn handle_input_event(event: Event) {
    // println!("device {:?}",event.device.DeviceRemovedEvent.device());
    match event {
        Keyboard(keyboard_event) => {
            // let keyBoardDevice = EventTrait::device(&keyboard_event);
            // println!("device {:?}",keyBoardDevice);
            // println!("device name{:?}",keyBoardDevice.name());
            // println!("device mode{:?}",keyBoardDevice.config_send_events_modes());
            // println!("device set mode{:?}",keyBoardDevice.config_send_events_set_mode(keyBoardDevice.config_send_events_modes()));
            
            
            println!("keyboard event {:?}\n", keyboard_event);
            let KeyboardEvent::Key(keyboard_key_event) = keyboard_event;
            let key = keyboard_key_event.key();
            println!("key {:?}", key);
            println!("key_state {:?}", keyboard_key_event.key_state());
            // input::config_send_events_set_mode(&input::SendEventsMode::DISABLED);
            // if let Some(keybd_key) = scan_code_to_key(key) {
            //     if keyboard_key_event.key_state() == KeyState::Pressed {
            //         if let Some(Bind::NormalBind(cb)) = KEYBD_BINDS.lock().unwrap().get(&keybd_key) {
            //             let cb = Arc::clone(cb);
            //             spawn(move || cb());
            //         };
            //     }
            // }
        }
        Pointer(pointer_event) => {
            // println!("pointer_event {:?}", pointer_event);
            // if let Button(button_event) = pointer_event {
            //     let button = button_event.button();
            //     if let Some(mouse_button) = match button {
            //         272 => Some(MouseButton::LeftButton),
            //         273 => Some(MouseButton::RightButton),
            //         274 => Some(MouseButton::MiddleButton),
            //         275 => Some(MouseButton::X1Button),
            //         276 => Some(MouseButton::X2Button),
            //         _ => None,
            //     } {
            //         if button_event.button_state() == ButtonState::Pressed {
            //             BUTTON_STATES.lock().unwrap().insert(mouse_button, true);
            //             if let Some(Bind::NormalBind(cb)) = MOUSE_BINDS.lock().unwrap().get(&mouse_button) {
            //                 let cb = Arc::clone(cb);
            //                 spawn(move || cb());
            //             };
            //         } else {
            //             BUTTON_STATES.lock().unwrap().insert(mouse_button, false);
            //         }
            //     }
            // }
        }
        randomEvent => {
            println!("Random EVent {:?}", randomEvent)
        }
    }
}