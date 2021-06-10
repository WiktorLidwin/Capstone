# Capstone

LightSwitch is an application allowing users to send keyboard and mouse events inbetween their devices. 

# Installation and Usage 

Download the repository and compile using cargo run --release or cargo build --release and locate the exe file in /target/release and run it.

The application will automatically discover peers on the same network. Once connected to the network, locate the 6 character long id on the device you wish to connect to 
and type in "Connectto {id}". Confirm on the second device. This will connect your device to the secondary one you wish to exchange events with. 

Type "devices" to get a list of all connected devices. To change a device name, go on the device and type "setname {name}". To create a set do "newset {name}". 
For a profile do "newprofile {set name} {profile name}"(names cannot have spaces*)

To edit profiles type "editprofile {set name} {profile name} {peripheral} {sender name} {list of receivers names}" (sender and receiver are the names set before hand) 
Windows will always have "mouse" and "keyboard" for peripherals, while Linux will have a number that can be found with the command "peripherals"

To run a set type "startset {setname}"

Other commands: "kick {peer name}" "disconnect" "setexit {list of exit key combination}" ex "setexit 29 42 35" "setcycle {list of cycle key combination}" 
"loadsets" "savesets" "sets" "connectfrom {id}" "mouserate {mouserate in hz(default 120)}" "subtopic" - gets current id "clearset"

# Libraries 
Rust, Libp2p + Floodsub(for p2p network), Winapi(Windows hooks), evdev(Linux hooks), cocoa + core graphics (Mac hooks)
