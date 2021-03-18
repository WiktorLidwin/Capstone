use evdev_rs::Device;
use std::fs::File;
use evdev;
use std::io::prelude::*;
use nix::unistd::Uid;
use nix::sys::ioctl;
// const SPI_IOC_MAGIC: u8 = b'k'; // Defined in linux/spi/spidev.h
// const SPI_IOC_TYPE_MODE: u8 = 1
// ioctl!(write tempdevice with SPI_IOC_MAGIC, SPI_IOC_TYPE_MODE; u8);

use std::os::unix::io::AsRawFd;
use input_linux;

use uinput::event::Event::{Controller, Relative};
use uinput::event::relative::Position::{X, Y};
use uinput::event::relative::Relative::Position;
use uinput::event::relative::Relative::Wheel;
use uinput::event::relative::Wheel::{Horizontal, Dial,Vertical};

pub use crossbeam_channel::{unbounded,Sender, Receiver};
pub use lazy_static::lazy_static;
pub use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::{thread, time};

#[path="../events.rs"]
mod events;
use crate::events::events::{KeyboardEvent, MouseEvent, LinuxEvent};
use crate::events::events::*;

fn main2(){
    if !Uid::effective().is_root() {
        panic!("You must run this executable with root permissions");
    }
    let mut devices: Vec<evdev::Device> = evdev::enumerate();
    for (i, d) in devices.iter().rev().enumerate() {
        
        println!("{}: {:?}", i, d.name());
    }

    print!("Select the device [0-{}]: ", devices.len());
    let _ = std::io::stdout().flush();
    let mut chosen = String::new();
    std::io::stdin().read_line(&mut chosen).unwrap();
    println!("device :{:?}",devices[chosen.trim().parse::<usize>().unwrap()]);
    let f = File::open("/dev/uinput").unwrap();     
    unsafe {
        let mut device = uinput::default().unwrap()
            .name("test").unwrap()
            .event(uinput::event::Event::All).unwrap()
            .event(Relative(Position(X))).unwrap()
		    .event(Relative(Position(Y))).unwrap()
            .event(Relative(Wheel(Horizontal))).unwrap()
            .event(Relative(Wheel(Dial))).unwrap()
            .event(Relative(Wheel(Vertical))).unwrap()
            .create().unwrap();

        let f = File::open("/dev/input/event".to_owned() + &chosen.trim()).unwrap();
        let mut d = Device::new().unwrap();
        d.set_fd(f).unwrap();
        
        d.grab(evdev_rs::GrabMode::Grab);
        loop {
            let a = d.next_event(evdev_rs::ReadFlag::NORMAL | evdev_rs::ReadFlag::BLOCKING);
            match a {
                Ok(k) => {
                    println!("event: event_type:{:?}, event_code{:?}, value{:?}", k.1.event_type, k.1.event_code, k.1.value);
                    let event = k.1.as_raw();
                    let bytes = device.write(event.type_.into(), event.code.into(), event.value);
                    device.synchronize().unwrap();
                    println!("bytes: {:?}", bytes);
                },
                Err(e) => (),
            }
        }
    }
}

// fn main(){
//     init();
// }


#[derive(Debug, Clone)]
pub struct LinuxDevice{
    pub name: String,
    pub id: i32,
    pub event_channel: (Sender<LinuxEvent>, Receiver<LinuxEvent>),
    pub blocking_channel: (Sender<bool>, Receiver<bool>),
}

pub static mut EXITKEYS:Vec<u32> = vec![];
pub static mut EXITKEYSDOWN:Vec<bool> = vec![];
pub static mut CYCLEKEYS:Vec<u32> = vec![];
pub static mut CYCLEKEYSDOWN:Vec<bool> = vec![];

pub unsafe fn setexitkeys(keys: Vec<u32>){
    // EXITKEYS = keys;
    EXITKEYS = vec![];
    EXITKEYSDOWN = vec![];
    for i in 0..EXITKEYS.len() {
        EXITKEYSDOWN.push(false);
        EXITKEYS.push(from_windows_keyboard_event(KeyboardEvent{vkCode: 0, scanCode: keys[i], flags: 0, time: 0}).code.into());
    }   
}

pub unsafe fn setcyclekeys(keys: Vec<u32>){
    // CYCLEKEYS = keys;
    CYCLEKEYS = vec![];
    CYCLEKEYSDOWN = vec![];
    for i in 0..keys.len() {
        CYCLEKEYS.push(from_windows_keyboard_event(KeyboardEvent{vkCode: 0, scanCode: keys[i], flags: 0, time: 0}).code.into());
        CYCLEKEYSDOWN.push(false);
    }
}

lazy_static! {
    pub static ref LINUX_DEVICES: Mutex<Vec<LinuxDevice>> = Mutex::new(vec![]);
    pub static ref SIM_DEVICE: Mutex<uinput::Device> = Mutex::new(uinput::default().unwrap()
        .name("test").unwrap()
        .event(uinput::event::Event::All).unwrap()
        .event(Relative(Position(X))).unwrap()
        .event(Relative(Position(Y))).unwrap()
        .event(Relative(Wheel(Horizontal))).unwrap()
        .event(Relative(Wheel(Dial))).unwrap()
        .event(Relative(Wheel(Vertical))).unwrap()
        .create().unwrap());
}

use std::ffi::CString;
use std::os::raw::c_char;

pub fn get_all_peripherals() -> Vec<LinuxDevice>{
    if !Uid::effective().is_root() {
        panic!("You must run this executable with root permissions");
    }
    let mut rawdevices: Vec<evdev::Device> = evdev::enumerate();
    let mut linuxdevices: Vec<LinuxDevice> =  vec![];
    linuxdevices.push(LinuxDevice{name: rawdevices[rawdevices.len()-1].name().clone().into_string().unwrap().clone(), id: 0,event_channel: unbounded(), blocking_channel: unbounded()});
    for (i, d) in rawdevices.iter().rev().skip(1).enumerate() {
        // println!("{}: {:?}", i, d.name());
        // println!("{}: {:?}", i, d.unique_name().clone().unwrap_or(CString::new("Hello, world!").expect("CString::new failed")));
        
        if !d.name().clone().into_string().unwrap().clone().contains(&linuxdevices[linuxdevices.len()-1].name){
            linuxdevices.push(LinuxDevice{name: d.name().clone().into_string().unwrap().clone(), id: (i+1) as i32, event_channel: unbounded(), blocking_channel: unbounded()});
        }
    }
    // for (i, d) in linuxdevices.iter().enumerate() {
    //     println!("{}: {:?}", i, d.name);
    // }
    // println!("{:?}", linuxdevices);
    linuxdevices
}
use std::time::Duration;
//15 is keyboard id
pub fn init(){
    handle_linux_devices_mutex();


    // println!("state: {:?}", LINUX_DEVICES.lock().unwrap());
    // println!("state: {:?}", get_linux_devices());
    let linux_devices = get_linux_devices();
    // println!("here?");
    // println!("linux_devices {:?}", linux_devices);
    // let _ = std::io::stdout().flush();
    // let mut chosen = String::new();
    // std::io::stdin().read_line(&mut chosen).unwrap();
    // println!("chosen {:?}", chosen);
    // let receiver = get_peripheral_receiver_with_id(chosen.clone().trim().to_string());
    // println!("chosen {:?}", chosen);
    // thread::spawn(move || {
    //     for event in receiver.iter() {
    //         println!("event:{:?}", event);
    //         write_to_sim_device(event);
    //     }
    // });
    // thread::spawn(move || {
    //     println!("chosen {:?}", chosen);
    //     thread::sleep(Duration::from_millis(1000)); // please do this :D
    //     grab_peripheral_with_id(chosen.clone().trim().to_string());
    // });
    // let receiver = get_peripheral_receiver_with_id("22".to_string());
    // thread::spawn(move || {
    //     for event in receiver.iter() {
    //         println!("event:{:?}", event);
    //         write_to_sim_device(event);
    //     }
    // });
    // thread::sleep(Duration::from_millis(1000)); // please do this :D
    // grab_peripheral_with_id("22".to_string());
    // println!("device :{:?}",devices[chosen.trim().parse::<usize>().unwrap()]);

}

// event:LinuxEvent { type_: 4, code: 4, value: 458792 }
// event:LinuxEvent { type_: 1, code: 28, value: 0 }

pub fn handle_linux_devices_mutex(){
    let mut state = LINUX_DEVICES.lock().unwrap();
    *state = get_all_peripherals();
}

pub fn get_linux_devices() -> Vec<LinuxDevice>{
    // println!("Here~");
    LINUX_DEVICES.lock().expect("cannot lock mutex").clone()
}

pub fn get_peripheral_id(name: String) -> String {
    if let Some(pos) = LINUX_DEVICES.lock().unwrap().iter().position(|device| device.name == name){
        return LINUX_DEVICES.lock().unwrap()[pos].id.clone().to_string()
    }
    "error".to_string()
} 


pub fn get_peripheral_pos(name: String) -> usize {
    if let Some(pos) = LINUX_DEVICES.lock().unwrap().iter().position(|device| device.name == name){
        return pos
    }
    0
} 

pub fn get_peripheral_pos_with_id(id: String) -> usize {
    let vec = get_linux_devices();
    if let Some(pos) = vec.into_iter().position(|device| device.id.to_string() == id){
        return pos
    }
    0
} 


pub fn get_peripheral_receiver(name: String) -> Receiver<LinuxEvent>{
    let index = get_peripheral_pos(name.clone().trim().to_string());
    LINUX_DEVICES.lock().unwrap()[index].event_channel.1.clone()
}

pub fn get_peripheral_receiver_with_id(id: String) -> Receiver<LinuxEvent>{
    let index = get_peripheral_pos_with_id(id.clone().trim().to_string());
    LINUX_DEVICES.lock().unwrap()[index].event_channel.1.clone()
}

pub fn get_peripheral_blocker(name: String) -> Sender<bool>{
    let index = get_peripheral_pos(name.clone().trim().to_string());
    LINUX_DEVICES.lock().unwrap()[index].blocking_channel.0.clone()
}

pub fn get_peripheral_blocker_with_id(id: String) -> Sender<bool>{
    let index = get_peripheral_pos_with_id(id.clone().trim().to_string());
    LINUX_DEVICES.lock().unwrap()[index].blocking_channel.0.clone()
}


pub fn grab_peripheral(name: String){
    let f = File::open("/dev/input/event".to_owned() + &get_peripheral_id(name.clone().trim().to_string())).unwrap();
    let mut d = Device::new().unwrap();
    d.set_fd(f).unwrap();
    // d.grab(evdev_rs::GrabMode::Grab);
    loop {
        let a = d.next_event(evdev_rs::ReadFlag::NORMAL | evdev_rs::ReadFlag::BLOCKING);
        match a {
            Ok(k) => {
                println!("event: event_type:{:?}, event_code{:?}, value{:?}", k.1.event_type, k.1.event_code, k.1.value);
                let index = get_peripheral_pos(name.clone().trim().to_string());
                LINUX_DEVICES.lock().unwrap()[index].event_channel.0.clone().send(LinuxEvent{type_: k.1.as_raw().type_, code: k.1.as_raw().code, value: k.1.value}).unwrap();
            },
            Err(e) => (),
        }
    }
}

pub fn grab_peripheral_with_id(id: String){
    println!("here! id {}", id);
    let f = File::open("/dev/input/event".to_owned() + &id.clone()).unwrap();
    let mut d = Device::new().unwrap();
    d.set_fd(f).unwrap();
    d.grab(evdev_rs::GrabMode::Ungrab);
    println!("test...");
    let index = get_peripheral_pos_with_id(id.clone().trim().to_string());
    let recv = LINUX_DEVICES.lock().unwrap()[index].blocking_channel.1.clone();
    loop {
        let a = d.next_event(evdev_rs::ReadFlag::NORMAL | evdev_rs::ReadFlag::BLOCKING);
        match a {
            Ok(k) => {
                // println!("event: event_type:{:?}, event_code{:?}, value{:?}", k.1.event_type, k.1.event_code, k.1.value);
                let index = get_peripheral_pos_with_id(id.clone().trim().to_string());
                LINUX_DEVICES.lock().unwrap()[index].event_channel.0.clone().send(LinuxEvent{type_: k.1.as_raw().type_, code: k.1.as_raw().code, value: k.1.value}).unwrap();
                unsafe{
                    if check_cycle_keys(&LinuxEvent{type_: k.1.as_raw().type_, code: k.1.as_raw().code, value: k.1.value}){
                        CYCLEPROGRAM
                            .0
                            .lock()
                            .expect("Failed to unlock Mutex")
                            .clone()
                            .send(true);
                    }
                    if check_exit_keys(&LinuxEvent{type_: k.1.as_raw().type_, code: k.1.as_raw().code, value: k.1.value}){
                        TERMINATEPROGRAM
                            .0
                            .lock()
                            .expect("Failed to unlock Mutex")
                            .clone()
                            .send(true);
                    }
                };
            },
            Err(e) => (),
        }
        let result = recv.try_recv();
        if !result.is_err(){
            let value = result.unwrap();
            if value{
                d.grab(evdev_rs::GrabMode::Grab);
            }else{
                d.grab(evdev_rs::GrabMode::Ungrab);
            }
            
            println!("result {:?}",value);
            println!("##################################################################################################");
        }
    }
}

// fn create_sim_device(){
//     let mut device = uinput::default().unwrap()
//             .name("test").unwrap()
//             .event(uinput::event::Event::All).unwrap()
//             .event(Relative(Position(X))).unwrap()
// 		    .event(Relative(Position(Y))).unwrap()
//             .event(Relative(Wheel(Horizontal))).unwrap()
//             .event(Relative(Wheel(Dial))).unwrap()
//             .event(Relative(Wheel(Vertical))).unwrap()
//             .create().unwrap();

// }

pub fn write_to_sim_device(event: LinuxEvent){
    // let event = k.1.as_raw();
    let bytes = SIM_DEVICE.lock().unwrap().write(event.type_.into(), event.code.into(), event.value);
    SIM_DEVICE.lock().unwrap().synchronize().unwrap();
}

pub fn reset_all_devices(){
    for device in get_linux_devices(){
        device.blocking_channel.0.clone().send(false).unwrap();
    }
}



pub fn get_exitkeys_recv() -> Receiver<bool>{
    TERMINATEPROGRAM
        .1
        .lock()
        .expect("Failed to unlock Mutex")
        .clone()
}

pub fn get_cyclekeys_recv() -> Receiver<bool>{
    CYCLEPROGRAM
        .1
        .lock()
        .expect("Failed to unlock Mutex")
        .clone()
}


lazy_static! {
    pub static ref TERMINATEPROGRAM: (Mutex<Sender<bool>>,Mutex<Receiver<bool>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    pub static ref CYCLEPROGRAM: (Mutex<Sender<bool>>,Mutex<Receiver<bool>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
}

unsafe fn check_cycle_keys(event:&LinuxEvent ) -> bool {
    // if event.code == evdev_rs::enums::EV_KEY::KEY_LEFTCTRL as u32 as u16{
    //     println!("ok so code is correct");
    // }else{
    //     println!("code is: {:?}    should be: {}",event.code, evdev_rs::enums::EV_KEY::KEY_LEFTCTRL as u32 as u16);
    // }
    if let Some(pos) = CYCLEKEYS.iter().position(|&x| x == event.code as u32 && 1 == event.type_){
        if event.value ==  0{
            for i in pos..CYCLEKEYSDOWN.len(){
                CYCLEKEYSDOWN[i] = false;
            }
        }else{
            if pos == 0 {
                CYCLEKEYSDOWN[pos] = true;
            }else if !CYCLEKEYSDOWN.iter().take(pos).map(|x| *x).collect::<Vec<bool>>().iter().any(|&x| x == false){
                CYCLEKEYSDOWN[pos] = true;
            }else{
                // println!("array {:?} code: {:?}", EXITKEYSDOWN,keyboard_event_struct.scanCode);
            }
            
        }
        if !CYCLEKEYSDOWN.iter().any(|&x| x == false){
            for i in 0..CYCLEKEYSDOWN.len(){
                CYCLEKEYSDOWN[i] = false;
            }
            return true
        }
    }
    false
}


unsafe fn check_exit_keys(event:&LinuxEvent ) -> bool {
    if let Some(pos) = EXITKEYS.iter().position(|&x| x == event.code as u32 && 1 == event.type_){
        if event.value ==  0{
            for i in pos..EXITKEYSDOWN.len(){
                // println!("CLEARCLEARCLEAR");
                EXITKEYSDOWN[i] = false;
            }
        }else{
            if pos == 0 {
                EXITKEYSDOWN[pos] = true;
            }else if !EXITKEYSDOWN.iter().take(pos).map(|x| *x).collect::<Vec<bool>>().iter().any(|&x| x == false){
                // println!("ININININININN");
                EXITKEYSDOWN[pos] = true;
            }else{
                // println!("array {:?} code: {:?}", EXITKEYSDOWN,keyboard_event_struct.scanCode);
            }
            
        }
        if !EXITKEYSDOWN.iter().any(|&x| x == false){
            for i in 0..EXITKEYSDOWN.len(){
                EXITKEYSDOWN[i] = false;
            }
            return true
        }
    }
    false
}

