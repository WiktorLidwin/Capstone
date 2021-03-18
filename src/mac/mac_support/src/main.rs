extern crate rdev;
use rdev::{listen, Event};
use rdev::{simulate, Button, EventType, Key, SimulateError};
use std::{thread, time};
use enigo::*;
use futures::executor::block_on;

fn callback(event: Event) {
    //yes this does work globally
    //test code, will print all actions in the console
    //println!("My callback {:?}", event); <- this line shows all mouse movement in the console, was for debugging purposes 
    match event.name {
        //debugging purposes, will sim the string returned by key press. for certain keys that dont have a corresponding alphanumeric character, 
        //like control/command, etc, it will give back ""
        Some(string) => println!("User wrote {:?}", string),
        None => (),
    }
}

fn sim(event_type: &EventType) {
    let delay = time::Duration::from_millis(20);
    match simulate(event_type) {
        Ok(()) => (),
        Err(SimulateError) => {
            println!("We could not sim {:?}", event_type);
        }
    }
    thread::sleep(delay);
}

fn block(flag: i32) {
    if flag==1 {
        //code to block input, i gotta find another crate to do this cuz the one im using didnt really help
    }
}

fn test() {
    for x in 0..20 {
        sim(&EventType::KeyRelease(Key::KeyS));
    }
}

fn main() {
    // let mut enigo = Enigo::new();
    thread::spawn(|| {
        test();
        thread::sleep(time::Duration::from_millis(1));
    });

    if let Err(error) = listen(callback) {
        println!("Error: {:?}", error)
    }   
    thread::sleep(time::Duration::from_millis(1));
}
