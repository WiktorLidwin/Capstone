// #[path = "./common.rs"]
use crate::windows::common::*;

pub const HOTKEY:u64 = 65;

lazy_static! {
    pub static ref MOUSE_EVENT_CHANNEL: (Mutex<Sender<MSLLHOOKSTRUCT>>, Mutex<Receiver<MSLLHOOKSTRUCT>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    pub static ref TERMINATEPROGRAM: (Mutex<Sender<bool>>,Mutex<Receiver<bool>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    pub static ref MOUSE_HHOOK: Lazy<AtomicPtr<HHOOK__>> = Lazy::new(AtomicPtr::default);
    pub static ref BLOCK_MOUSE:bool = true;
}

pub fn receive_mouse_event(){
    thread::spawn(move || {
        let recv = MOUSE_EVENT_CHANNEL.1.lock().expect("Failed to unlock Mutex");
        for mouse_event_struct in recv.iter() {
            println!("key_event_struct: pt: x: {:?},y: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",mouse_event_struct.pt.x,mouse_event_struct.pt.y, mouse_event_struct.mouseData,mouse_event_struct.flags,mouse_event_struct.time,mouse_event_struct.dwExtraInfo);
        }
    });
    // thread::spawn(move || {
    //     let recv = TERMINATEPROGRAM.1.lock().expect("Failed to unlock Mutex");
    //     for hook_ptr in recv.iter() {
    //         unset_hook(&*MOUSE_HHOOK);
    //         process::exit(1);
    //     }
    // });
    set_hook(WH_MOUSE_LL,&*MOUSE_HHOOK,mouse_hook_callback);
    unsafe{
        let mut msg: MSG = MaybeUninit::zeroed().assume_init();
        GetMessageW(&mut msg, 0 as HWND, 0, 0);
        println!("{:?},{:?}", msg.hwnd, msg.message);
    }
    // unsafe {
    //     let hook_id = user32::SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_callback), std::ptr::null_mut(), 0);
    //     let mut msg: MSG = MaybeUninit::zeroed().assume_init();
    //     GetMessageW(&mut msg, 0 as HWND, 0, 0);
    // };
}

pub unsafe extern "system" fn mouse_hook_callback(code: i32, wParam: WPARAM, lParam: LPARAM) -> LRESULT {
    println!("hererere");
    let mouse_event_struct = (*(lParam as *const MSLLHOOKSTRUCT));
    MOUSE_EVENT_CHANNEL
        .0
        .lock()
        .expect("Failed to unlock Mutex")
        .send(mouse_event_struct)
        .expect("Receiving end of KEYBOARD_EVENT_CHANNEL was closed");
    // if mouse_event_struct.vkCode == 65{
    //     TERMINATEPROGRAM
    //     .0
    //     .lock()
    //     .expect("Failed to unlock Mutex")
    //     .send(true)
    //     .expect("Receiving end of KEYBOARD_EVENT_CHANNEL was closed");
    // }
    if *BLOCK_MOUSE {
        return 1
    }else {
        return 0
    }
}
