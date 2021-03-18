// #[path = "./common.rs"]
use crate::windows::common::*;
#[path="../events.rs"]
mod events;
use crate::events::events::{KeyboardEvent, MouseEvent, LinuxEvent};
use crate::events::events::*;

pub static mut BLOCK_MOUSE:bool = false;
pub static mut FROZEN_MOUSE_POINT:(i32,i32) = (0,0);
pub static mut SET_FROZEN_MOUSE_POINT:bool = false;

lazy_static! {
    pub static ref MOUSE_EVENT_CHANNEL: (Mutex<Sender<MouseEvent>>, Mutex<Receiver<MouseEvent>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    pub static ref TERMINATEPROGRAM: (Mutex<Sender<bool>>,Mutex<Receiver<bool>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    pub static ref MOUSE_HHOOK: Lazy<AtomicPtr<HHOOK__>> = Lazy::new(AtomicPtr::default);
    
}
pub unsafe fn set_block_mouse(b:bool){
    BLOCK_MOUSE = b;
}

pub fn get_mouse_recv() -> Receiver<MouseEvent>{
    MOUSE_EVENT_CHANNEL
        .1
        .lock()
        .expect("Failed to unlock Mutex")
        .clone()
}

pub unsafe fn get_frozen_mouse_point() -> (i32,i32) {
    FROZEN_MOUSE_POINT
}


pub fn receive_mouse_event(){
    // thread::spawn(move || {
    //     let recv = MOUSE_EVENT_CHANNEL.1.lock().expect("Failed to unlock Mutex");
    //     for mouse_event_struct in recv.iter() {
    //         println!("key_event_struct: pt: x: {:?},y: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",mouse_event_struct.pt.x,mouse_event_struct.pt.y, mouse_event_struct.mouseData,mouse_event_struct.flags,mouse_event_struct.time,mouse_event_struct.dwExtraInfo);
    //     }
    // });
    // thread::spawn(move || {
    //     let recv = TERMINATEPROGRAM.1.lock().expect("Failed to unlock Mutex");
    //     for hook_ptr in recv.iter() {
    //         unset_hook(&*MOUSE_HHOOK);
    //         process::exit(1);
    //     }
    // });
   
    unsafe{ 
        SET_FROZEN_MOUSE_POINT = false;
        FROZEN_MOUSE_POINT = pos();
        set_hook(WH_MOUSE_LL,&*MOUSE_HHOOK,mouse_hook_callback);
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

fn wParamToFlag(wParam:WPARAM) -> DWORD{
    return match wParam as u32 {
        WM_LBUTTONDOWN => MOUSEEVENTF_LEFTDOWN,
        WM_RBUTTONDOWN => MOUSEEVENTF_RIGHTDOWN,
        WM_MBUTTONDOWN => MOUSEEVENTF_MIDDLEDOWN,
        WM_XBUTTONDOWN => MOUSEEVENTF_XDOWN,
        WM_LBUTTONUP => MOUSEEVENTF_LEFTUP,
        WM_RBUTTONUP => MOUSEEVENTF_RIGHTUP,
        WM_MBUTTONUP => MOUSEEVENTF_MIDDLEUP,
        WM_XBUTTONUP => MOUSEEVENTF_XUP,
        WM_MOUSEWHEEL => MOUSEEVENTF_WHEEL,
        WM_MOUSEHWHEEL => MOUSEEVENTF_HWHEEL,
        _ => 0,
    }
}

pub fn pos() -> (i32, i32) {
    unsafe {
        let mut point = MaybeUninit::uninit();
        GetCursorPos(point.as_mut_ptr());
        let point = point.assume_init();
        (point.x, point.y)
    }
}
pub fn move_rel(dx: i32, dy: i32) {
    let (x, y) = pos();
    move_abs(x + dx, y + dy);
}

pub fn move_abs(x: i32, y: i32) {
    unsafe {
        SetCursorPos(x, y);
    }
}

pub fn revert_mouse(){
    unset_hook(&*MOUSE_HHOOK);
}

pub unsafe extern "system" fn mouse_hook_callback(code: i32, wParam: WPARAM, lParam: LPARAM) -> LRESULT {
    
    let mouse_event_struct = (*(lParam as *const MSLLHOOKSTRUCT));
    // println!("pos: {:?}",pos());
    // println!("Freeze pos: {:?}",FROZEN_MOUSE_POINT);
    // println!("recieved pos {:?}",(mouse_event_struct.pt.x,mouse_event_struct.pt.y));
    // if !SET_FROZEN_MOUSE_POINT{
    //     SET_FROZEN_MOUSE_POINT = true;
    //     FROZEN_MOUSE_POINT = (mouse_event_struct.pt.x,mouse_event_struct.pt.y);
    // }
    // if let Some(event) = match wParam as u32 {
    //     WM_LBUTTONDOWN => {println!("WM_LBUTTONDOWN"); None},
    //     WM_RBUTTONDOWN => {println!("WM_RBUTTONDOWN"); None},
    //     WM_MBUTTONDOWN => {println!("WM_MBUTTONDOWN"); None},
    //     WM_XBUTTONDOWN => {
    //         let llhs = &*(lParam as *const MSLLHOOKSTRUCT);

    //         match HIWORD(llhs.mouseData) {
    //             XBUTTON1 => {println!("XBUTTON1"); None},
    //             XBUTTON2 => {println!("XBUTTON2"); None},
    //             _ => None,
    //         }
    //     },
    //     _ => Some(1),
    // } {

    // }
    if mouse_event_struct.dwExtraInfo == 1{
        return 0
    }

    let (x,y) = pos();
    // MOUSE_EVENT_CHANNEL  
    //     .0
    //     .lock()
    //     .expect("Failed to unlock Mutex")
    //     .send(MouseEvent{
    //         pt: (mouse_event_struct.pt.x-x,mouse_event_struct.pt.y-y),
    //         mouseData: mouse_event_struct.mouseData,
    //         flags: wParamToFlag(wParam),
    //         time: 0,
    //     })
    //     .expect("Receiving end of KEYBOARD_EVENT_CHANNEL was closed");
    // FROZEN_MOUSE_POINT = (mouse_event_struct.pt.x,mouse_event_struct.pt.y);
    MOUSE_EVENT_CHANNEL  
        .0
        .lock()
        .expect("Failed to unlock Mutex")
        .send(MouseEvent{
            pt: (mouse_event_struct.pt.x - x,mouse_event_struct.pt.y - y),
            mouseData: mouse_event_struct.mouseData,
            flags: wParamToFlag(wParam),
            time: 0,
        })
        .expect("Receiving end of KEYBOARD_EVENT_CHANNEL was closed");
    FROZEN_MOUSE_POINT = (mouse_event_struct.pt.x,mouse_event_struct.pt.y);
    if !BLOCK_MOUSE {
        // FROZEN_MOUSE_POINT = (mouse_event_struct.pt.x,mouse_event_struct.pt.y);
        return 0
    }else { 
        
        return 1
    }
}

pub fn send_mouse_input(flags: u32, data: u32, dx: i32, dy: i32) {
    let mut input = INPUT {
        type_: INPUT_MOUSE,
        u: unsafe {
            transmute_copy(&MOUSEINPUT {
                dx,
                dy,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 1,
            })
        },
    };
    unsafe { SendInput(1, &mut input as LPINPUT, size_of::<INPUT>() as c_int) };
}
