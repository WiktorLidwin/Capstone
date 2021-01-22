// #[path = "./common.rs"]
use crate::windows::common::*;

pub const HOTKEY:u32 = 65;
pub static mut BLOCK_KEYBOARD:bool = true;
lazy_static! {
    pub static ref KEYBOARD_EVENT_CHANNEL: (Mutex<Sender<KBDLLHOOKSTRUCT>>, Mutex<Receiver<KBDLLHOOKSTRUCT>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    pub static ref TERMINATEPROGRAM: (Mutex<Sender<bool>>,Mutex<Receiver<bool>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    pub static ref KEYBD_HHOOK: Lazy<AtomicPtr<HHOOK__>> = Lazy::new(AtomicPtr::default);
    
}
pub fn get_keyboard_recv() -> Receiver<KBDLLHOOKSTRUCT>{
    KEYBOARD_EVENT_CHANNEL
        .1
        .lock()
        .expect("Failed to unlock Mutex")
        .clone()
}
pub unsafe fn set_block_keyboard(b:bool){
     BLOCK_KEYBOARD = b;
}
pub fn receive_keyboard_event(){
    // thread::spawn(move || {
    //     let recv = KEYBOARD_EVENT_CHANNEL.1.lock().expect("Failed to unlock Mutex");
    //     for key_event_struct in recv.iter() {
    //         println!("key_event_struct: code: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",key_event_struct.vkCode, key_event_struct.scanCode,key_event_struct.flags,key_event_struct.time,key_event_struct.dwExtraInfo);
    //     }
    // });
    thread::spawn(move || {
        let recv = TERMINATEPROGRAM.1.lock().expect("Failed to unlock Mutex");
        for _ in recv.iter() {
            unset_hook(&*KEYBD_HHOOK);
            process::exit(1);
        }
    });
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

pub unsafe extern "system" fn keyboard_hook_callback(code: i32, wParam: WPARAM, lParam: LPARAM) -> LRESULT {
    let mut keyboard_event_struct = (*(lParam as *const KBDLLHOOKSTRUCT));
    if keyboard_event_struct.vkCode == HOTKEY{
        TERMINATEPROGRAM
        .0
        .lock()
        .expect("Failed to unlock Mutex")
        .send(true)
        .expect("Receiving end of TERMINATEPROGRAM was closed");
    }
    // if keyboard_event_struct.vkCode == 66{
    //     if keyboard_event_struct.flags == 0 {
    //        send_keybd_input(KEYEVENTF_SCANCODE , 67); 
    //     }  
    // }
    println!("IMPORTANT!!!: code: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",keyboard_event_struct.vkCode, keyboard_event_struct.scanCode,keyboard_event_struct.flags,keyboard_event_struct.time,keyboard_event_struct.dwExtraInfo);
        
    if keyboard_event_struct.dwExtraInfo == 1 || !BLOCK_KEYBOARD {
        // println!("sent event!");
        return 0
    }else { 
        // if wParam as u32 == WM_KEYDOWN {
        //     keyboard_event_struct.flags = 0;
        // }else{
        //     keyboard_event_struct.flags = 128;
        // }
        KEYBOARD_EVENT_CHANNEL  
        .0
        .lock()
        .expect("Failed to unlock Mutex")
        .send(keyboard_event_struct)
        .expect("Receiving end of KEYBOARD_EVENT_CHANNEL was closed");
        
        
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
                wVk: key_code as u16,
                wScan: scan_code as u16,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 1,
            })//MapVirtualKeyW(key_code, 0) as u16
        },
    };
    unsafe { SendInput(1, &mut input as LPINPUT, size_of::<INPUT>() as c_int) };
}

// fn listen_to_keyboard_event(){

// }
