use rdev::{listen, Event, simulate, Button, EventType, Key, SimulateError};
use std::{thread, time};
use std::time::Duration;

fn callback(event: Event) {
    println!("My callback {:?}", event);
    match event.name {
        Some(string) => println!("User wrote {:?}", string),
        None => (),
    }
}

fn send(event_type: &EventType) {
    let delay = time::Duration::from_millis(20);
    match simulate(event_type) {
        Ok(()) => (),
        Err(SimulateError) => {
            println!("We could not send {:?}", event_type);
        }
    }
    thread::sleep(delay);
}

fn main() {
    thread::spawn(|| {
        if let Err(error) = listen(callback) {
            println!("Error: {:?}", error)
        }
    });

    let b: bool = true;

    while b {
        send(&EventType::MouseMove { x: 0.0, y: 0.0 });
        send(&EventType::MouseMove { x: 400.0, y: 400.0 }); //this is gonna spazz out your mouse, its for testing
    //     send(&EventType::KeyPress(Key::KeyS));
    //     send(&EventType::KeyRelease(Key::KeyS));    
    //     send(&EventType::ButtonRelease(Button::Right));
    //     send(&EventType::Wheel {
    //         delta_x: 0,
    //         delta_y: 1,
    //     });
    }
    // you can use the send command here to simulate input, but it needs to be in a separate thread from the callback function 
    // or else the callback function is gonna block anything else after it
}