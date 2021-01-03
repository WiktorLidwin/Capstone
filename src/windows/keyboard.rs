// #[path = "./common.rs"]
use crate::windows::common::*;

pub const HOTKEY:u32 = 65;

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
    pub static ref BLOCK_KEYBOARD:bool = true;
}

pub fn receive_keyboard_event(){
    thread::spawn(move || {
        let recv = KEYBOARD_EVENT_CHANNEL.1.lock().expect("Failed to unlock Mutex");
        for key_event_struct in recv.iter() {
            println!("key_event_struct: code: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",key_event_struct.vkCode, key_event_struct.scanCode,key_event_struct.flags,key_event_struct.time,key_event_struct.dwExtraInfo);
        }
    });
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
    let keyboard_event_struct = (*(lParam as *const KBDLLHOOKSTRUCT));
    KEYBOARD_EVENT_CHANNEL
        .0
        .lock()
        .expect("Failed to unlock Mutex")
        .send(keyboard_event_struct)
        .expect("Receiving end of KEYBOARD_EVENT_CHANNEL was closed");
    if keyboard_event_struct.vkCode == HOTKEY{
        TERMINATEPROGRAM
        .0
        .lock()
        .expect("Failed to unlock Mutex")
        .send(true)
        .expect("Receiving end of TERMINATEPROGRAM was closed");
    }
    if *BLOCK_KEYBOARD {
        return 1
    }else {
        return 0
    }
}

// fn listen_to_keyboard_event(){

// }