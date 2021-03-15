use lazy_static::lazy_static;
use rdev::{simulate, Button, EventType, Key, SimulateError};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::{thread, time};

// #[path = "/windows/keyboard.rs"]
// use crate::keyboard::*;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use crate::windows::*;

#[cfg(target_os = "windows")]
fn main(){
    // thread::spawn(move || {
    //     receive_keyboard_event(); 
    // });
    // receive_mouse_event();
    set_block_keyboard(true);

    receive_keyboard_event();
}
// #[cfg(target_os = "linux")]
// mod linux;
// #[cfg(target_os = "linux")]
// pub use crate::linux::*;
// use std::io::{stdin, stdout, Write};
// use termion::event::Key;
// use termion::input::TermRead;
// use termion::raw::IntoRawMode;
// use inputbot::{BlockInput::*, KeybdKey::*, MouseButton::*, *};
use inputbot::{ KeybdKey::*,MouseButton::*,*};
// use enigo::{Enigo, Key, KeyboardControllable, *};
// use device_query::{DeviceQuery, DeviceState, MouseState, Keycode};
fn send(event_type: &EventType) {
    let delay = time::Duration::from_millis(20);
    match simulate(event_type) {
        Ok(()) => (),
        Err(SimulateError) => {
            println!("We could not send {:?}", event_type);
        }
    }
    // Let ths OS catchup (at least MacOS)
    thread::sleep(delay);
}
#[cfg(target_os = "linux")]
fn main(){

    println!("LINUX BABY!");
    AKey.bind(|| {   
        // while AKey.is_pressed(){
            // let mut enigo = Enigo::new();
            // enigo.key_sequence_parse("a");
            println!("pain"); 
        // }
        send(&EventType::KeyPress(Key::KeyA));
        send(&EventType::KeyPress(Key::KeyS));
        thread::sleep(time::Duration::from_millis(500));
        send(&EventType::KeyRelease(Key::KeyA));
        send(&EventType::KeyRelease(Key::KeyS));
        AKey.press();
            thread::sleep(time::Duration::from_millis(50));
            AKey.release();
    });
    BKey.bind(|| {
        AKey.unbind();
        AKey.bind(|| { 
            AKey.press();
            thread::sleep(time::Duration::from_millis(50));
            AKey.release();
         })
    });
    // RKey.bind(|| KeySequence("Sample text").send());

    // Move mouse.
    QKey.bind(|| MouseCursor.move_rel(10, 10));

    NumLockKey.bind(|| {
        while NumLockKey.is_toggled() {
            LShiftKey.press();
            WKey.press();
            thread::sleep(time::Duration::from_millis(50));
            WKey.release();
            LShiftKey.release();
        }
    });
    // KKey.bind(|| MouseCursor::move_rel(10, 10));
    // // Move mouse.
    // QKey.bind(|| MouseCursor::move_rel(10, 10));
    // WKey.bind(|| println!("pain2"));
    // AKey.blockable_bind(|| {
    //     if LShiftKey.is_pressed() {
    //         Block
    //     } else {
    //         DontBlock
    //     }
    // });
    // // Call this to start listening for bound inputs.
    handle_input_events();
    // temp();
    // let device_state = DeviceState::new();
    // let mouse: MouseState = device_state.get_mouse();
    // println!("Current Mouse Coordinates: {:?}", mouse.coords);
    // let keys: Vec<Keycode> = device_state.get_keys();
    // println!("Is A pressed? {}", keys.contains(&Keycode::A));
}
// fn temp() {
//     let stdin = stdin();
//     //setting up stdout and going into raw mode
//     let mut stdout = stdout().into_raw_mode().unwrap();
//     //printing welcoming message, clearing the screen and going to left top corner with the cursor
//     write!(stdout, r#"{}{}ctrl + q to exit, ctrl + h to print "Hello world!", alt + t to print "termion is cool""#, termion::cursor::Goto(1, 1), termion::clear::All)
//             .unwrap();
//     stdout.flush().unwrap();

//     //detecting keydown events
//     for c in stdin.keys() {
//         //clearing the screen and going to top left corner
//         write!(
//             stdout,
//             "{}{}",
//             termion::cursor::Goto(1, 1),
//             termion::clear::All
//         )
//         .unwrap();

//         //i reckon this speaks for itself
//         match c.unwrap() {
//             Key::Ctrl('h') => println!("Hello world!"),
//             Key::Ctrl('q') => break,
//             Key::Alt('t') => println!("termion is cool"),
//             Key::Char('q') => (),
//             Key::Char('a') => (),
            
//             _ => (),
//         }

//         stdout.flush().unwrap();
//     }
// }


// fn main(){
//     println!("No clue wgar ur running bro");
// }

// lazy_static! {
//     static ref EVENT_CHANNEL: (Mutex<Sender<Event>>, Mutex<Receiver<Event>>) = {
//         let (send, recv) = channel();
//         (Mutex::new(send), Mutex::new(recv))
//     };
// }

// fn send_event(event: Event) {
//     EVENT_CHANNEL
//         .0
//         .lock()
//         .expect("Failed to unlock Mutex")
//         .send(event)
//         .expect("Receiving end of EVENT_CHANNEL was closed");
// }

// fn main2() {
//     println!("main2");
//     // spawn new thread because listen blocks
//     let _listener = thread::spawn(move || {
//         listen(send_event).expect("Could not listen");
//     });

//     let recv = EVENT_CHANNEL.1.lock().expect("Failed to unlock Mutex");
//     // let mut events = Vec::new();
//     let mut freeze_mouse = false;
//     let mut last_position = (0.0, 0.0);
//     for event in recv.iter() {
//         // events.push(event);
//         match event.name{
//             Some(string) =>match string.as_str(){
//                 "c"=>{
//                     freeze_mouse = !freeze_mouse;
//                     println!("freeze_mouse: {}, mousepositon : {:?}",freeze_mouse,last_position);
//                 },
//                 _ =>println!("char: {:?}",event.event_type)
//             },
//             None => ()
//         }match event.event_type{
//             EventType::MouseMove{x,y} =>{
//                 if freeze_mouse{
//                     match simulate(&EventType::MouseMove { x: last_position.0, y:last_position.1 }) {
//                         Ok(()) => (),
//                         Err(SimulateError) => {
//                             println!("We could not send");
//                         }
//                     }
//                 }else{
//                     last_position =(x,y);
//                 }
//             },  
//             _ =>()
//         }
//         // println!("Received {} events", events.len());
//     }
// }
// use inputbot::{KeybdKey::*, MouseButton::*, *};
// use std::{thread::sleep, time::Duration};


// fn temp1() {
//     let keys = vec![BackspaceKey,
//     TabKey,
//     EnterKey,
//     EscapeKey,
//     SpaceKey,
//     HomeKey,
//     LeftKey,
//     UpKey,
//     RightKey,
//     DownKey,
//     InsertKey,
//     DeleteKey,
//     Numrow0Key,
//     Numrow1Key,
//     Numrow2Key,
//     Numrow3Key,
//     Numrow4Key,
//     Numrow5Key,
//     Numrow6Key,
//     Numrow7Key,
//     Numrow8Key,
//     Numrow9Key,
//     AKey,
//     BKey,
//     CKey,
//     DKey,
//     EKey,
//     FKey,
//     GKey,
//     HKey,
//     IKey,
//     JKey,
//     KKey,
//     LKey,
//     MKey,
//     NKey,
//     OKey,
//     PKey,
//     QKey,
//     RKey,
//     SKey,
//     TKey,
//     UKey,
//     VKey,
//     WKey,
//     XKey,
//     YKey,
//     ZKey,
//     Numpad0Key,
//     Numpad1Key,
//     Numpad2Key,
//     Numpad3Key,
//     Numpad4Key,
//     Numpad5Key,
//     Numpad6Key,
//     Numpad7Key,
//     Numpad8Key,
//     Numpad9Key,
//     F1Key,
//     F2Key,
//     F3Key,
//     F4Key,
//     F5Key,
//     F6Key,
//     F7Key,
//     F8Key,
//     F9Key,
//     F10Key,
//     F11Key,
//     F12Key,
//     F13Key,
//     F14Key,
//     F15Key,
//     F16Key,
//     F17Key,
//     F18Key,
//     F19Key,
//     F20Key,
//     F21Key,
//     F22Key,
//     F23Key,
//     F24Key,
//     NumLockKey,
//     ScrollLockKey,
//     CapsLockKey,
//     LShiftKey,
//     RShiftKey,
//     LControlKey,
//     RControlKey,
//     OtherKey(65 as u64),
//     OtherKey(0x41),
//     ];
//     let mouseKeys = vec![LeftButton,
//     MiddleButton,
//     RightButton,
//     X1Button,
//     X2Button];
//     // // Autorun for videogames.
//     // NumLockKey.bind(|| {
//     //     while NumLockKey.is_toggled() {
//     //         LShiftKey.press();
//     //         WKey.press();
//     //         sleep(Duration::from_millis(50));
//     //         WKey.release();
//     //         LShiftKey.release();
//     //     }
//     // });

//     // // Rapidfire for videogames.
//     // RightButton.bind(|| {
//     //     while RightButton.is_pressed() {
//     //         LeftButton.press();
//     //         sleep(Duration::from_millis(50));
//     //         LeftButton.release();
//     //     }
//     // });

//     // // Send a key sequence.
//     // RKey.bind(|| KeySequence("Sample text").send());

//     // // Move mouse.
//     // QKey.bind(|| MouseCursor::move_rel(10, 10));
//     // for k in keys{
//     //     k.block_bind(|| println!("pain"));
//     // }
//     // OtherKey(0x41).press();
//     // OtherKey(65 as u64).press();
//     // KeybdKey::from(65 as u64).block_bind(|| println!("pain"));
//     // AKey.block_bind(|| println!("pain"));
    
//     // OtherKey(18 as u64).blockable_bind(|| {println!("test"); BlockInput::Block});
//     // OtherKey(91 as u64).block_bind(|| println!("pain"));
//     // OtherKey(92 as u64).block_bind(|| println!("pain"));
//     // for i in 0..256{
//     //     KeybdKey::from(i as u64).block_bind(|| println!("pain"));
//     // }
//     for k in mouseKeys{
//         k.block_bind(|| println!("pain"));
//     }
//     KeybdKey::from(18 as u64).block_bind(|| println!("pain"));
//     // OtherKey(18 as u64).block_bind(|| println!("pain"));
//     // WKey.block_bind(|| println!("pain"));
//     // println!("{}",inputbot::KEYBD_BINDS );
//     // Call this to start listening for bound inputs.
//     handle_input_events();
// }
// extern crate user32;
// extern crate winapi;
// use std::{
//     mem::{size_of, transmute_copy, MaybeUninit},
//     ptr::null_mut,
//     sync::atomic::AtomicPtr,
// };
// use winapi::{
//     ctypes::*,
//     shared::{minwindef::*, windef::*},
//     um::winuser::*,
// };
// use once_cell::sync::Lazy;
// static KEYBD_HHOOK: Lazy<AtomicPtr<HHOOK__>> = Lazy::new(AtomicPtr::default);
// static MOUSE_HHOOK: Lazy<AtomicPtr<HHOOK__>> = Lazy::new(AtomicPtr::default);
// // const WH_KEYBOARD_LL: i32 = 13;

// fn main3() {
//     // thread::spawn(move || {
//     //     main2();
//     // });
//     unsafe {
//         println!("keybd {:?}  mouse {:?}", KEYBD_HHOOK,MOUSE_HHOOK);
//         let hook_id =
//             user32::SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_callback), std::ptr::null_mut(), 0);
//         // let hook_id2 =
//         //     user32::SetWindowsHookExW(WH_MOUSE_LL, Some(hook_callback), std::ptr::null_mut(), 0);
        
//             let mut msg: MSG = { MaybeUninit::zeroed().assume_init() };
//         GetMessageW(&mut msg, 0 as HWND, 0, 0);
//         // Don't forget to release the hook eventually
//         // user32::UnhookWindowsHookEx(hook_id);
//     }
    
// }

// unsafe extern "system" fn hook_callback(code: i32, wParam: u64, lParam: i64) -> i64 {
//     // println!("code {} wPrarm {:?} lParam {:?}", code, wParam, lParam);
//     // println!(" {:?} ",(KeybdKey::from(u64::from(
//     //     (*(lParam as *const KBDLLHOOKSTRUCT)).vkCode,
//     // ))));
//     // if let OtherKey(key) = (KeybdKey::from(u64::from(//TODO should be i64 i think
//     //     (*(lParam as *const KBDLLHOOKSTRUCT)).vkCode,
//     // ))){
//     //     if key == 164{
//     //         println!("ALTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT");
//     //         return 1;
//     //     }else{
//     //         println!("SADDDDDDDDDDDDDDDDDDDDD")
//     //     }
//     // }
//     let key_event_struct = (*(lParam as *const KBDLLHOOKSTRUCT));
//     println!("key_event_struct: code: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",key_event_struct.vkCode, key_event_struct.scanCode,key_event_struct.flags,key_event_struct.time,key_event_struct.dwExtraInfo);
//     // let llhs = &*(lParam as *const MSLLHOOKSTRUCT);
//     // println!("data x: {:?} y: {:?}",llhs.pt.x,llhs.pt.y);
//     // match HIWORD(llhs.mouseData) {
//     //     XBUTTON1 => println!("btn1 {:?}",XBUTTON1),
//     //     XBUTTON2 => println!("btn2 {:?}",XBUTTON2),
//     //     _ => (),
//     // }
//     0
// }