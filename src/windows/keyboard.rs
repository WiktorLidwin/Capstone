// #[path = "./common.rs"]
use crate::windows::common::*;
use crate::windows::mouse::*;
use serde::{Deserialize, Serialize};
pub static mut EXITKEYS:Vec<u32> = vec![];
pub static mut EXITKEYSDOWN:Vec<bool> = vec![];
pub static mut CYCLEKEYS:Vec<u32> = vec![];
pub static mut CYCLEKEYSDOWN:Vec<bool> = vec![];
pub static mut BLOCK_KEYBOARD:bool = true;
#[path="../events.rs"]
mod events;
use crate::events::events::{KeyboardEvent, MouseEvent, LinuxEvent};
use crate::events::events::*;

lazy_static! {
    pub static ref KEYBOARD_EVENT_CHANNEL: (Mutex<Sender<KeyboardEvent>>, Mutex<Receiver<KeyboardEvent>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    pub static ref TERMINATEPROGRAM: (Mutex<Sender<bool>>,Mutex<Receiver<bool>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    pub static ref CYCLEPROGRAM: (Mutex<Sender<bool>>,Mutex<Receiver<bool>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    pub static ref KEYBD_HHOOK: Lazy<AtomicPtr<HHOOK__>> = Lazy::new(AtomicPtr::default);
    
}
pub fn get_keyboard_recv() -> Receiver<KeyboardEvent>{
    KEYBOARD_EVENT_CHANNEL
        .1
        .lock()
        .expect("Failed to unlock Mutex")
        .clone()
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

pub unsafe fn set_block_keyboard(b:bool){
    BLOCK_KEYBOARD = b;
}

pub fn revert_keyboard(){
    unset_hook(&*KEYBD_HHOOK);
}

pub unsafe fn setexitkeys(keys: Vec<u32>){
    EXITKEYS = keys;
    EXITKEYSDOWN = vec![];
    for i in 0..EXITKEYS.len() {
        EXITKEYSDOWN.push(false);
    }   
}

pub unsafe fn setcyclekeys(keys: Vec<u32>){
    CYCLEKEYS = keys;
    CYCLEKEYSDOWN = vec![];
    for i in 0..CYCLEKEYS.len() {
        CYCLEKEYSDOWN.push(false);
    }
}



pub fn receive_keyboard_event(){
    // unsafe{
    //     EXITKEYSDOWN = vec![];
    //     for i in 0..EXITKEYS.len() {
    //         EXITKEYSDOWN.push(false);
    //     }
    //     CYCLEKEYSDOWN = vec![];
    //     for i in 0..CYCLEKEYS.len() {
    //         CYCLEKEYSDOWN.push(false);
    //     }
    // };
    
    // thread::spawn(move || {
    //     let recv = KEYBOARD_EVENT_CHANNEL.1.lock().expect("Failed to unlock Mutex");
    //     for key_event_struct in recv.iter() {
    //         println!("key_event_struct: code: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",key_event_struct.vkCode, key_event_struct.scanCode,key_event_struct.flags,key_event_struct.time,key_event_struct.dwExtraInfo);
    //     }
    // });
    // thread::spawn(move || {
    //     let recv = TERMINATEPROGRAM.1.lock().expect("Failed to unlock Mutex");
    //     for _ in recv.iter() {
    //         revert_keyboard();
    //         revert_mouse();
    //     }
    // });
    set_hook(WH_KEYBOARD_LL,&*KEYBD_HHOOK,keyboard_hook_callback);
    unsafe{
        let mut msg: MSG = MaybeUninit::zeroed().assume_init();
        GetMessageW(&mut msg, 0 as HWND, 0, 0);
    }
    // unsafe {
    //     let hook_id = user32::SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_callback), std::ptr::null_mut(), 0);
    //     let mut msg: MSG = MaybeUninit::zeroed().assume_init();
    //     GetMessageW(&mut msg, 0 as HWND, 0, 0);
    // };
}

unsafe fn check_cycle_keys(keyboard_event_struct:&KeyboardEvent ) -> bool {
    if let Some(pos) = CYCLEKEYS.iter().position(|&x| x == keyboard_event_struct.scanCode){
        if (keyboard_event_struct.flags >> 7) % 2 ==  1{
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
            println!("OUTOUTOUTOUTOUTOUTOUTOUT2222222222222");
            for i in 0..CYCLEKEYSDOWN.len(){
                CYCLEKEYSDOWN[i] = false;
            }
            return true
        }
    }
    false
}

unsafe fn check_exit_keys(keyboard_event_struct:&KeyboardEvent ) -> bool {
    if let Some(pos) = EXITKEYS.iter().position(|&x| x == keyboard_event_struct.scanCode){
        if (keyboard_event_struct.flags >> 7) % 2 ==  1{
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
            println!("OUTOUTOUTOUTOUTOUTOUTOUT");
            for i in 0..EXITKEYSDOWN.len(){
                EXITKEYSDOWN[i] = false;
            }
            return true
        }
    }
    false
}

pub unsafe extern "system" fn keyboard_hook_callback(code: i32, wParam: WPARAM, lParam: LPARAM) -> LRESULT {
    let mut keyboard_event_struct = (*(lParam as *const KBDLLHOOKSTRUCT));// keyboard_event_struct.vkCode == HOTKEY
    let mut extendedkey = 0;
    if keyboard_event_struct.flags % 2 ==  1{
        // println!("exteneded");
        extendedkey += 57344;
    }
    let new_event = KeyboardEvent{
        vkCode: keyboard_event_struct.vkCode,
        scanCode: keyboard_event_struct.scanCode + extendedkey,
        flags: keyboard_event_struct.flags,
        time: 0,
    };
    
    if check_exit_keys(&new_event){
        TERMINATEPROGRAM
        .0
        .lock()
        .expect("Failed to unlock Mutex")
        .send(true)
        .expect("Receiving end of TERMINATEPROGRAM was closed");
    }
    if check_cycle_keys(&new_event){
        CYCLEPROGRAM
        .0
        .lock()
        .expect("Failed to unlock Mutex")
        .send(true)
        .expect("Receiving end of TERMINATEPROGRAM was closed");
    }
    // if keyboard_event_struct.vkCode == 66{
    //     println!("EXITKEYSDOWN {:?}  EXITKEYS  {:?} CYCLEKEYS {:?}  CYCLEKEYSDOWN {:?}",EXITKEYSDOWN, EXITKEYS,CYCLEKEYS,CYCLEKEYSDOWN)
    // }
    // println!("IMPORTANT!!!: code: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",keyboard_event_struct.vkCode, keyboard_event_struct.scanCode,keyboard_event_struct.flags,keyboard_event_struct.time,keyboard_event_struct.dwExtraInfo);
    //Comment 
    if keyboard_event_struct.dwExtraInfo == 1{
        return 0
    }
    KEYBOARD_EVENT_CHANNEL  
        .0
        .lock()
        .expect("Failed to unlock Mutex")
        .send(new_event)
        .expect("Receiving end of KEYBOARD_EVENT_CHANNEL was closed");
        
    if  !BLOCK_KEYBOARD {
        // println!("sent event!");
        return 0
    }else { 
        // if wParam as u32 == WM_KEYDOWN {
        //     keyboard_event_struct.flags = 0;
        // }else{
        //     keyboard_event_struct.flags = 128;
        // }
        
        
        return 1
    }
    
    // if *BLOCK_KEYBOARD {
    //     return 1
    // }
}

pub fn send_keybd_input(scan_code: u32, key_code: u32, flags: u32) {
    let mut input = INPUT {
        type_: INPUT_KEYBOARD,
        u: unsafe {
            transmute_copy(&KEYBDINPUT {
                wVk:  key_code as u16,
                wScan: scan_code as u16,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 1,
            })//MapVirtualKeyW(u64::from(key_code) as u32, 0) as u16
        },
    };
    unsafe { SendInput(1, &mut input as LPINPUT, size_of::<INPUT>() as c_int) };
}

// fn listen_to_keyboard_event(){

// }
