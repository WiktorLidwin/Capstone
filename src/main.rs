use chrono;
use crossbeam;
use libp2p::{
    core::upgrade,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{MdnsEvent, TokioMdns},
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    NetworkBehaviour, PeerId, Transport,
};
use local_ipaddress;
use log::{error, info};
use mac_address;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::num::Wrapping;
use std::path::PathBuf;
use std::{thread, time};
use tokio::{fs, io::AsyncBufReadExt, sync::mpsc};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("Capstone"));


static KEYLENGTH: usize = 6;
static mut KEYBOARDAVARAGE: Vec<i64> = vec![];
static mut MOUSEAVARAGE: Vec<i64> = vec![];
static UDPSOCKET: Lazy<UdpSocket> = Lazy::new(|| {
    UdpSocket::bind(local_ipaddress::get().unwrap() + ":0").expect("couldn't bind to address")
});
static mut UDPMAP: Lazy<HashMap<String, String>> = Lazy::new(|| HashMap::new());
static mut DEVICENAMESMAP: Lazy<HashMap<String, Device>> = Lazy::new(|| HashMap::new());
static mut PUBLISHEDUDP: bool = false;
static mut CONNECTEDTOPEERS: bool = false;
static mut TRUSTEDDEVICES: Vec<[u8; 6]> = vec![];
static mut HOST: bool = true;
// static mut AUTOCONNECT: bool = true;
// static mut CURRENTSET:Set;
// static mut KEYBOARDDESTINATION: Vec<String> = vec![];
// static mut MOUSEDESTINATION: Vec<String> = vec![];
static mut SETS: Vec<Set> = vec![];


lazy_static! {
    static ref TERMINATETHREADS: (Mutex<Sender<bool>>, Mutex<Receiver<bool>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    static ref KEYBOARD_RECEIVERS: Mutex<Vec<String>> = Mutex::new(vec![]);
    static ref MOUSE_RECEIVERS: Mutex<Vec<String>> = Mutex::new(vec![]);
    static ref CURRENTSET: Mutex<String> = Mutex::new("".into());
    static ref SUBTOPIC: Mutex<Topic> = Mutex::new(Topic::new(""));
    static ref PERIPHERAL_RECEIVERS: Mutex<HashMap<String, Vec<String>>> =
        Mutex::new(HashMap::new());
    static ref AUTOCONNECT: Mutex<Vec<u8>> = Mutex::new(vec![]);
    static ref MOUSE_RATE:Mutex<i32> =  Mutex::new(120);
    static ref MOUSE_BUFFER:Mutex<(i32,i32)> =  Mutex::new((0,0));
    static ref LASTMOUSEINSTANT:Mutex<Instant> =  Mutex::new(Instant::now());
    
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use crate::windows::*;

#[path="events.rs"]
mod events;
use crate::events::events::{KeyboardEvent, MouseEvent, LinuxEvent};
use crate::events::events::*;

// #[path = "../../TicTacToeStructs.rs"]
// mod TicTacToeStructs;
// use crate::TicTacToeStructs::TicTacToeStructs::Message;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use crate::linux::*;

fn searchDeviceName(device_name: String) -> String {
    unsafe {
        for (peer_id, device) in &*DEVICENAMESMAP {
            if device.name == device_name {
                return peer_id.clone().to_owned();
            }
        }
        return "".to_string();
    }
}

fn searchDeviceMacAddress(mac_address: Vec<u8>) -> String {
    unsafe {
        for (peer_id, device) in &*DEVICENAMESMAP {
            if device.mac_addr.clone().to_vec() == mac_address {
                return peer_id.clone().to_owned();
            }
        }
        return "".to_string();
    }
}

fn getlocalPath() -> PathBuf {
    let mut dir = env::current_exe().unwrap();
    dir.pop();
    println!("{}", dir.display());
    dir
}

#[derive(Debug, Serialize, Deserialize)]
struct Device {
    //struct with name, mac addr, and OS maybe more
    name: String,
    mac_addr: [u8; 6],
    os: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StartMessage {
    set: String,
    profile: String,
}


// impl Device {
//     fn editName(self, peer_id:String, new_name: String,sender: mpsc::UnboundedSender<(Message, i32)>) {
//         let msg = Message {
//             sender: PEER_ID.to_string(),
//             header: "EditDeviceName".to_string(),
//             data: peer_id+" "+&new_name,
//             receiver: vec![],
//         };
//         if let Err(e) = sender.send((msg,0)) {
//             error!("error sending response via channel, {}", e);
//         }
//     }
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Profile {
    // mouse_target: HashMap<String, Vec<String>>,
    // keyboard_target: HashMap<String, Vec<String>>,
    peripheral_receivers: HashMap<String,HashMap<String, Vec<String>>>,
    id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Set {
    profiles: Vec<Profile>,
    id: String,
    exitkeys: Vec<u32>,
    cyclekeys: Vec<u32>,
    profile_order: Vec<String>,
}

impl Set {
    fn new(id: String, sender: mpsc::UnboundedSender<(Message, i32)>) -> bool {
        if Set::findSet(id.clone()) != -1 {
            return false;
        }
        let set = Set {
            profiles: vec![],
            id,
            exitkeys: vec![29, 42, 34],
            cyclekeys: vec![29, 42, 35],
            profile_order: vec![],
        };
        unsafe {
            SETS.push(set);
        };
        updateSet(sender.clone());
        return true;
        // &PROFILES[PROFILES.len()-1]
    }
    fn newProfile(
        set_id: String,
        profile_id: String,
        sender: mpsc::UnboundedSender<(Message, i32)>,
    ) -> bool {
        // self.profiles.rooms.iter().find(|&x| x.id == id);
        println!("set_id: {:?}, profile_id: {:?}", set_id, profile_id);
        unsafe {
            // let mut set = SETS[Set::findSet(set_id) as usize];
            let index = Set::findSet(set_id.clone());
            if index == -1 {
                return false;
            }
            if SETS[index as usize].findProfile(profile_id.clone()) != -1 {
                return false;
            }
            let profile = Profile {
                peripheral_receivers: HashMap::new(),
                id: profile_id.clone(),
            };
            SETS[index as usize].profiles.push(profile);
            SETS[index as usize].profile_order.push(profile_id);
        };
        updateSet(sender.clone());
        true
        // &PROFILES[PROFILES.len()-1]
    }
    fn view(&self) -> Vec<Profile> {
        return self.profiles.clone();
    }
    fn findSet(id: String) -> i32 {
        unsafe {
            if let Some(pos) = SETS.iter().position(|x| x.id == id) {
                return pos as i32;
            }
            return -1;
        }
    }
    fn findProfile(&mut self, profile_id: String) -> i32 {
        if let Some(pos) = self.profiles.iter().position(|x| x.id == profile_id) {
            return pos as i32;
        }
        -1
    }
    fn removeProfile(&mut self, profile_id: String, sender: mpsc::UnboundedSender<(Message, i32)>) {
        self.profiles.retain(|x| x.id != profile_id);
        updateSet(sender.clone());
    }
    fn getProfile(&mut self, profile_id: String) -> Option<&Profile> {
        let temp = self.profiles.iter().find(|&x| x.id == profile_id);
        temp
    }
    fn delete(&mut self, sender: mpsc::UnboundedSender<(Message, i32)>) {
        unsafe {
            SETS.retain(|x| x.id != self.id);
        };
        updateSet(sender.clone());
    }
    fn editProfile(
        set_id: String,
        profile_id: String,
        peripheral: String,
        sender_id: String,
        receivers: Vec<String>,
        sender: mpsc::UnboundedSender<(Message, i32)>,
    ) {
        unsafe {
            if let Some(pos) = SETS[Set::findSet(set_id.clone()) as usize]
                .profiles
                .iter()
                .position(|profile| profile.id == profile_id)
            {
                let z = &mut SETS[Set::findSet(set_id) as usize].profiles[pos];
                z.edit(peripheral, sender_id, receivers);
            }
        }
        updateSet(sender.clone());
    }
    async fn loadFromDefaultFile() -> Vec<SaveSet> {
        let mut local_path = getlocalPath();
        local_path.push("sets.json");
        if local_path.exists() {
            let data = fs::read_to_string(local_path)
                .await
                .expect("Unable to read file");
            let temp = serde_json::from_str::<Vec<SaveSet>>(&data).unwrap();
            return temp;
        }
        println!("couldnt open");
        return vec![];
    }
    async fn saveToDefaultFile() {
        unsafe {
            let mut local_path = getlocalPath();
            local_path.push("sets.json");
            println!("final path {:?}", local_path);
            let converted_sets = convert_sets_to_save();
            let data = serde_json::to_string(&converted_sets).expect("can jsonify response");
            fs::write(local_path, data)
                .await
                .expect("Unable to write file");
        }
    }
    fn cycleProfiles(set_id: String, sender: mpsc::UnboundedSender<(Message, i32)>) {
        unsafe {
            SETS[Set::findSet(set_id.clone()) as usize]
                .profile_order
                .rotate_left(1);
            println!(
                "cycled to {:?}",
                SETS[Set::findSet(set_id.clone()) as usize].profile_order[0].clone()
            );
            let msg = Message {
                sender: PEER_ID.to_string(),
                header: "StartSet".to_string(),
                data: serde_json::to_string(&StartMessage {
                    set: set_id.clone(),
                    profile: SETS[Set::findSet(set_id.clone()) as usize].profile_order[0].clone(),
                })
                .expect("can jsonify request"),
                receiver: vec![],
            };
            if let Err(e) = sender.send((msg, 1)) {
                error!("error sending response via channel, {}", e);
            } else {
                thread::spawn(move || Set::startSet(set_id, sender.clone()));
            }
        }
    }
    fn startProfile(
        set_id: String,
        profile_id: String,
        sender: mpsc::UnboundedSender<(Message, i32)>,
    ) {
        unsafe {
            while SETS[Set::findSet(set_id.clone()) as usize].profile_order[0] != profile_id {
                SETS[Set::findSet(set_id.clone()) as usize]
                    .profile_order
                    .rotate_left(1);
            }
        };
        thread::spawn(move || {
            Set::startSet(set_id, sender);
        });
    }
    fn startSet(set_id: String, sender: mpsc::UnboundedSender<(Message, i32)>) {
        //TODO here
        println!("in startset");
        println!("someone explain... {:?}", set_id);
        set_currentset(set_id.clone());
        println!("set1");
        unsafe {
            println!("set2");
            setexitkeys(SETS[Set::findSet(set_id.clone()) as usize].exitkeys.clone());
            println!("set3");
            setcyclekeys(
                SETS[Set::findSet(set_id.clone()) as usize]
                    .cyclekeys
                    .clone(),
            );
            println!("set4");
            SETS[Set::findSet(set_id.clone()) as usize]
                .getProfile(SETS[Set::findSet(set_id) as usize].profile_order[0].clone())
                .unwrap()
                .load(sender.clone());
        };
        println!("finished startset");
        println!("{:?}",PERIPHERAL_RECEIVERS
            .lock()
            .expect("Failed to unlock Mutex"));
    
        // broadcastSet(set_id.clone(), sender.clone());
        // SETS[Set::findSet(set_id) as usize].profiles[0].load();
    }
    fn setexit(set_id: String, keys: Vec<u32>, sender: mpsc::UnboundedSender<(Message, i32)>) {
        unsafe {
            SETS[Set::findSet(set_id.clone()) as usize].exitkeys = keys;
        }
        updateSet(sender.clone());
    }
    fn setcycle(set_id: String, keys: Vec<u32>, sender: mpsc::UnboundedSender<(Message, i32)>) {
        unsafe {
            SETS[Set::findSet(set_id.clone()) as usize].exitkeys = keys;
        }
        updateSet(sender.clone());
    }
    fn editorder(
        set_id: String,
        order: Vec<String>,
        sender: mpsc::UnboundedSender<(Message, i32)>,
    ) {
        unsafe {
            SETS[Set::findSet(set_id.clone()) as usize].profile_order = order;
        }
    }
    fn getfirstprofile(set_id: String) -> String {
        unsafe {
            return SETS[Set::findSet(set_id.clone()) as usize].profile_order[0].clone();
        }
    }
}

fn updateSet(sender: mpsc::UnboundedSender<(Message, i32)>) {
    println!("sending update sets...");
    unsafe {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "UpdateSets".to_string(),
            data: serde_json::to_string(&SETS).expect("can jsonify request"),
            receiver: vec![],
        };
        if let Err(e) = sender.clone().send((msg, 1)) {
            error!("error sending response via channel, {}", e);
        }
    }
}

fn broadcastSet(set_id: String, sender: mpsc::UnboundedSender<(Message, i32)>) {
    unsafe {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "Set".to_string(),
            data: serde_json::to_string(&SETS[Set::findSet(set_id) as usize])
                .expect("can jsonify request"),
            receiver: vec![],
        };
        sender.send((msg, 1)).expect("sent msg");
    }
}

impl Profile {
    fn edit(&mut self, peripheral: String, sender: String, receivers: Vec<String>) {
        // if peripheral == "mouse" {
        //     self.mouse_target.insert(sender, receivers);
        // } else if peripheral == "keyboard" {
        //     self.keyboard_target.insert(sender, receivers);
        // } else {
        //     return;
        // }
        let mut temp_hashmap = HashMap::new();
        temp_hashmap.insert(sender, receivers);
        self.peripheral_receivers.insert(
            peripheral,
            temp_hashmap
        );
        // hash_map[sender] = receivers;
        // ig make nicknames
    } //TODO
    #[cfg(target_os = "windows")]
    fn load(&self, sender: mpsc::UnboundedSender<(Message, i32)>) {
        println!("loading...");
        // if let Some(val) = self.mouse_target.get(&PEER_ID.to_string()) {
        //     println!("SWAPPING MOUSE {:?}", val.clone());
        //     // swap_mouse(sender.clone(), val.clone());
        //     if val.clone().iter().any(|i| (*i) == PEER_ID.to_string()) {
        //         set_mouse_block(false);
        //     } else {
        //         set_mouse_block(true);
        //     }
        //     set_mouse_recivers(val.clone());
        // } else {
        //     println!("NOTSWAPPING MOUSE");
        //     // swap_mouse(sender.clone(), vec![PEER_ID.to_string()]);
        //     set_mouse_recivers(vec![]);
        //     set_mouse_block(false);
        // }
        // if let Some(val) = self.keyboard_target.get(&PEER_ID.to_string()) {
        //     println!("SWAPPING Keyboard {:?}", val.clone());
        //     // swap_keyboard(sender.clone(), val.clone());
        //     if val.clone().iter().any(|i| (*i) == PEER_ID.to_string()) {
        //         set_keyboard_block(false);
        //     } else {
        //         set_keyboard_block(true);
        //     }
        //     set_keyboard_recivers(val.clone());
        // } else {
        //     println!("NOTSWAPPING KEYBOARD");
        //     // swap_keyboard(sender.clone(), vec![PEER_ID.to_string()]);//TODO test swapping and closing....
        //     set_keyboard_block(false);
        //     set_keyboard_recivers(vec![]);
        // }
        for (peripheral, targets) in &self.peripheral_receivers{
            println!("peripheral {}: {:?}", peripheral, targets);
            if let Some(val) = targets.get(&PEER_ID.to_string()) {
                println!("inside {:?}",val);
                if val.clone().iter().any(|i| (*i) == PEER_ID.to_string()) {
                    set_peripheral_block(peripheral.clone(),false);
                } else {
                    set_peripheral_block(peripheral.clone(),true);
                }
                PERIPHERAL_RECEIVERS
                    .lock()
                    .expect("Failed to unlock Mutex")
                    .insert(peripheral.clone(), val.clone());
            }else{
                set_peripheral_block(peripheral.clone(),false);
                PERIPHERAL_RECEIVERS
                    .lock()
                    .expect("Failed to unlock Mutex")
                    .insert(peripheral.clone(), vec![]);
            }

        }
        // let mouse_RECEIVERS = self.mouse_target.get(&PEER_ID.to_string()).unwrap().clone();
        // let keyboard_RECEIVERS = self.keyboard_target.get(&PEER_ID.to_string()).unwrap().clone();
        // swap_keyboard(sender.clone(),keyboard_RECEIVERS);
    }
    #[cfg(target_os = "linux")]
    fn load(&self, sender: mpsc::UnboundedSender<(Message, i32)>) {
        println!("in loading....");
        reset_all_devices();
        for (peripheral, targets) in &self.peripheral_receivers{
            println!("peripheral {}: {:?}", peripheral, targets);
            if let Some(val) = targets.get(&PEER_ID.to_string()) {
                println!("inside {:?}",val);
                if val.clone().iter().any(|i| (*i) == PEER_ID.to_string()) {
                    println!("not blocking ");
                    set_peripheral_block(peripheral.clone(),false);
                } else {
                    println!("blocking ");
                    set_peripheral_block(peripheral.clone(),true);
                }
                PERIPHERAL_RECEIVERS
                    .lock()
                    .expect("Failed to unlock Mutex")
                    .insert(peripheral.clone(), val.clone());
            }else{
                set_peripheral_block(peripheral.clone(),false);
                PERIPHERAL_RECEIVERS
                    .lock()
                    .expect("Failed to unlock Mutex")
                    .insert(peripheral.clone(), vec![]);
            }

        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SaveSet {
    profiles: Vec<SaveProfile>,
    id: String,
    exitkeys: Vec<u32>,
    cyclekeys: Vec<u32>,
    profile_order: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SaveProfile {
    // mouse_target: HashMap<String, Vec<String>>,
    // keyboard_target: HashMap<String, Vec<String>>,
    peripheral_receivers: HashMap<String,HashMap<String, Vec<String>>>,
    id: String,
}

fn from_mac_addr_to_string(mac_addr: Vec<u8>) -> String {
    mac_addr.iter().map(|x| x.to_string()+",").collect()
}

fn from_string_to_mac_addr(string: String) -> Vec<u8> {
    string.split(",").filter(|x| x != &"").map(|x| x.parse::<u8>().unwrap()).collect()
}

fn convert_sets_to_save() -> Vec<SaveSet> {
    let mut SAVESETS: Vec<SaveSet> = vec![];
    unsafe {
        for set in &SETS {
            let mut profiles: Vec<SaveProfile> = vec![];
            for profile in &set.profiles {
                let mut save_profile: SaveProfile = SaveProfile {
                    peripheral_receivers: HashMap::new(),
                    id: profile.id.clone(),
                };
                // for (key, value) in profile.mouse_target.iter() {
                //     save_profile.mouse_target.insert(
                //         String::from_utf8(DEVICENAMESMAP.get(key).expect(key).mac_addr.clone().to_vec()).unwrap(),
                //         value
                //             .clone()
                //             .into_iter()
                //             .map(|x| String::from_utf8(DEVICENAMESMAP.get(&x.clone()).unwrap().mac_addr.clone().to_vec()).unwrap())
                //             .collect(),
                //     );
                // }
                // for (key, value) in profile.keyboard_target.iter() {
                //     save_profile.keyboard_target.insert(
                //         String::from_utf8(DEVICENAMESMAP.get(key).expect(key).mac_addr.clone().to_vec()).unwrap(),
                //         value
                //             .clone()
                //             .into_iter()
                //             .map(|x| String::from_utf8(DEVICENAMESMAP.get(&x.clone()).unwrap().mac_addr.clone().to_vec()).unwrap())
                //             .collect(),
                //     );
                // }
                for (key, value) in profile.peripheral_receivers.iter() {
                    let mut temp_hashmap = HashMap::new();
                    for (key2, value2) in value.iter() {
                        println!("key2: {:?}, value2: {:?}", key,value2);
                        temp_hashmap.insert(
                            from_mac_addr_to_string(DEVICENAMESMAP.get(key2).expect(key2).mac_addr.clone().to_vec()),
                            value2
                                .clone()
                                .into_iter()
                                .map(|x| from_mac_addr_to_string(DEVICENAMESMAP.get(&x.clone()).expect(&x).mac_addr.clone().to_vec()))
                                .collect(),
                        );
                    }
                    save_profile.peripheral_receivers.insert(
                        key.clone(),
                        temp_hashmap
                    );
                }
                profiles.push(save_profile);
            }
            let save_set: SaveSet = SaveSet {
                profiles,
                id: set.id.clone(),
                exitkeys: set.exitkeys.clone(),
                cyclekeys: set.cyclekeys.clone(),
                profile_order: set.profile_order.clone(),
            };
            SAVESETS.push(save_set);
        }
    }
    println!("Savesets {:?}",SAVESETS);
    SAVESETS
}
fn from_save_sets(save_sets: Vec<SaveSet>) -> Vec<Set> {
    let mut Sets: Vec<Set> = vec![];
    unsafe {
        for set in &save_sets {
            let mut profiles: Vec<Profile> = vec![];
            for profile in &set.profiles {
                let mut save_profile: Profile = Profile {
                    peripheral_receivers: HashMap::new(),
                    id: profile.id.clone(),
                };
                for (key, value) in profile.peripheral_receivers.iter() {
                    let mut temp_hashmap = HashMap::new();

                    for (key2, value2) in value.iter() {
                        let peer_id = searchDeviceMacAddress(from_string_to_mac_addr(key2.clone()));
                        if peer_id != "".to_string() {
                            temp_hashmap.insert(
                                peer_id,
                                value2
                                    .clone()
                                    .into_iter()
                                    .map(|x| searchDeviceMacAddress(from_string_to_mac_addr(x.clone())))
                                    .collect(),
                            );
                        }
                    }
                    save_profile.peripheral_receivers.insert(
                        key.clone(),
                        temp_hashmap
                    );
                }
                // for (key, value) in profile.mouse_target.iter() {
                //     let peer_id = searchDeviceMacAddress(key.clone().as_bytes().to_vec());
                //     if peer_id != "".to_string() {
                //         save_profile.mouse_target.insert(
                //             peer_id,
                //             value
                //                 .clone()
                //                 .into_iter()
                //                 .map(|x| searchDeviceMacAddress(x.clone().as_bytes().to_vec()))
                //                 .collect(),
                //         );
                //     }
                // }
                // for (key, value) in profile.keyboard_target.iter() {
                //     let peer_id = searchDeviceMacAddress(key.clone().as_bytes().to_vec());
                //     if peer_id != "".to_string() {
                //         save_profile.keyboard_target.insert(
                //             peer_id,
                //             value
                //                 .clone()
                //                 .into_iter()
                //                 .map(|x| searchDeviceMacAddress(x.clone().as_bytes().to_vec()))
                //                 .collect(),
                //         );
                //     }
                // }

                profiles.push(save_profile);
            }
            let save_set: Set = Set {
                profiles,
                id: set.id.clone(),
                exitkeys: set.exitkeys.clone(),
                cyclekeys: set.cyclekeys.clone(),
                profile_order: set.profile_order.clone(),
            };
            Sets.push(save_set);
        }
    }
    Sets
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    sender: String,
    header: String,
    data: String,
    receiver: Vec<String>,
}

#[derive(Debug)]
enum EventType {
    Response((Message, i32)),
    Input(String),
}

#[derive(NetworkBehaviour)]
struct RecipeBehaviour {
    floodsub: Floodsub,
    mdns: TokioMdns,
    #[behaviour(ignore)]
    response_sender: mpsc::UnboundedSender<(Message, i32)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SaveTrustedDevices {
    mac_addresses: Vec<[u8; 6]>,
}
use slice_as_array::slice_to_array_clone;
use std::convert::TryInto;
use std::path::Path;

impl NetworkBehaviourEventProcess<FloodsubEvent> for RecipeBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(msg) => {
                // println!("getting msg!");
                if let Ok(resp) = serde_json::from_slice::<Message>(&msg.data) {
                    if resp.receiver.contains(&PEER_ID.to_string()) {
                        if resp.header == "Test".to_string() {
                            println!("perfect. Data: {:?} ", resp.data);
                        } else if resp.header == "KeyboardEvent".to_string() {
                            handle_KeyboardEvent(resp);
                        } else if resp.header == "MouseEvent".to_string() {
                            handle_MouseEvent(resp);
                        } else if resp.header == "Ping".to_string() {
                            handle_Ping(resp, self.response_sender.clone());
                        } else if resp.header == "Pong".to_string() {
                            handle_Pong(resp);
                        } else if resp.header == "PublishUDP" {
                            handle_PublishUDP(resp, self.response_sender.clone());
                        } else if resp.header == "RespondwithUDP" {
                            handleNewUDPSocket(resp.sender.clone(), resp.data);
                        } else if resp.header == "Connect" {
                            handle_Connect(resp, self.response_sender.clone());
                        } else if resp.header == "RespondConnect" {
                            handle_RespondConnect(resp);
                        } else if resp.header == "UpdateSets" {
                            println!("updating sets...");
                            handle_UpdateSets(resp);
                        } else if resp.header == "StartSet" {
                            handle_StartSet(resp, self.response_sender.clone());
                        } else if resp.header == "Unswap" {
                            unswap(resp.data);
                        } else if resp.header == "ConnectKey" {
                            handle_ConnectKey(resp, self.response_sender.clone());
                        } else if resp.header == "ErrorConnectKey" {
                            handle_ErrorConnectKey(resp, self.response_sender.clone());
                        } else if resp.header == "AttemptConnectSubTopic" {
                            handle_AttemptConnectSubTopic(resp, self.response_sender.clone());
                        } else if resp.header == "ConnectSubTopic" {
                            handle_ConnectSubTopic(
                                resp,
                                self.response_sender.clone(),
                                &mut self.floodsub,
                            );
                        } else if resp.header == "SuccessfulConnect" {
                            handle_SuccessfulConnect(self.response_sender.clone());
                        } else if resp.header == "TrustedDevices" {
                            handle_TrustedDevices(resp)
                        }else if resp.header == "AutoConnectAttempt" {
                            handle_AutoConnectAttempt(resp,self.response_sender.clone());
                        }else if resp.header == "AutoConnectConfirm" {
                            handle_AutoConnectConfirm(resp,self.response_sender.clone(),&mut self.floodsub);
                        }else if resp.header == "AutoConnectSuccess" {
                            handle_AutoConnectSuccess(self.response_sender.clone());
                        }else if resp.header == "ChangeName" {
                            handle_ChangeName(resp);
                        }
                        // resp.data.iter().for_each(|r| info!("{:?}", r));
                    }
                }
            }
            _ => (),
        }
    }
}

fn handle_ChangeName(resp: Message){
    if let Ok(device) = serde_json::from_str::<Device>(&resp.data) {
        unsafe {
            DEVICENAMESMAP.insert(resp.sender.to_string(), device);
        }
    }
}

#[cfg(target_os = "windows")]
fn handle_KeyboardEvent(resp: Message) {
    // println!("got keyboard event");/
    if let Ok(keyboard_event_struct) = serde_json::from_str::<KeyboardEvent>(&resp.data) {
        let dt = chrono::prelude::Local::now();
        let milliseconds: i64 = dt.timestamp_millis();
        unsafe { KEYBOARDAVARAGE.push(milliseconds - keyboard_event_struct.time) };
        if (keyboard_event_struct.flags >> 7) % 2 ==  1{
            // println!("mouse up!");
            send_keybd_input(
                0,
                keyboard_event_struct.vkCode,
                KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
            );
        }else{
            // println!("mouse down?");
            send_keybd_input(
                0,
                keyboard_event_struct.vkCode,
                KEYEVENTF_UNICODE,
            );
        }
        
        // if (keyboard_event_struct.flags >> 7) % 2 ==  1{
        //     // println!("mouse up!");
        //     if keyboard_event_struct.flags % 2 ==  1{
        //         send_keybd_input(
        //             keyboard_event_struct.scanCode,
        //             keyboard_event_struct.vkCode,
        //             KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY,
        //         );
        //     }else{
        //         send_keybd_input(
        //             keyboard_event_struct.scanCode,
        //             keyboard_event_struct.vkCode,
        //             KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP,
        //         );
        //     }
            
        // }else{
        //     // println!("mouse down?");
        //     if keyboard_event_struct.flags % 2 ==  1{
        //         send_keybd_input(
        //             keyboard_event_struct.scanCode,
        //             keyboard_event_struct.vkCode,
        //             KEYEVENTF_SCANCODE |KEYEVENTF_EXTENDEDKEY ,
        //         );
        //     }else{
        //         send_keybd_input(
        //             keyboard_event_struct.scanCode,
        //             keyboard_event_struct.vkCode,
        //             KEYEVENTF_SCANCODE,
        //         );
        //     }
        // }
        


        // if keyboard_event_struct.vkCode == 66 {
        //     println!("AAAAA :D")
        // }
        // if keyboard_event_struct.flags == 128 {
        //     send_keybd_input(
        //         keyboard_event_struct.scanCode,
        //         keyboard_event_struct.vkCode,
        //         KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP,
        //     );
        // } else if keyboard_event_struct.flags == 0 {
        //     send_keybd_input(
        //         keyboard_event_struct.scanCode,
        //         keyboard_event_struct.vkCode,
        //         KEYEVENTF_SCANCODE,
        //     );
        // } else if keyboard_event_struct.flags == 129 {
        //     send_keybd_input(
        //         0,
        //         keyboard_event_struct.vkCode,
        //         KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
        //     );
        // } else if keyboard_event_struct.flags == 1 {
        //     send_keybd_input(0, keyboard_event_struct.vkCode, KEYEVENTF_UNICODE);
        // } else if keyboard_event_struct.flags == 32 {
        //     //make sure
        //     send_keybd_input(0, keyboard_event_struct.vkCode, KEYEVENTF_UNICODE);
        // } else if (keyboard_event_struct.flags >> 5) % 2 == 1 {
        //     //make sure
        //     send_keybd_input(0, keyboard_event_struct.vkCode, KEYEVENTF_UNICODE);
        // } else if (keyboard_event_struct.flags >> 7) % 2 == 1 {
        //     //make sure
        //     send_keybd_input(
        //         0,
        //         keyboard_event_struct.vkCode,
        //         KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
        //     );
        // } else {
        //     println!("SOMETGHING DIFFERENT  {:?}", keyboard_event_struct.flags);
        //     send_keybd_input(
        //         0,
        //         keyboard_event_struct.vkCode,
        //         KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
        //     );
        // }
        //unhandled events : 160,161,33 (33 and 161 r together idfk 160)
    }
    if let Ok(peripheral_event_struct) = serde_json::from_str::<LinuxEvent>(&resp.data) {
        let keyboard_event_struct = from_linux_keyboard_event(peripheral_event_struct);
        if (keyboard_event_struct.flags >> 7) % 2 ==  1{
            // println!("mouse up!");
            if keyboard_event_struct.flags % 2 ==  1{
                send_keybd_input(
                    keyboard_event_struct.scanCode,
                    keyboard_event_struct.vkCode,
                    KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY,
                );
            }else{
                send_keybd_input(
                    keyboard_event_struct.scanCode,
                    keyboard_event_struct.vkCode,
                    KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP,
                );
            }
            
        }else{
            // println!("mouse down?");
            if keyboard_event_struct.flags % 2 ==  1{
                send_keybd_input(
                    keyboard_event_struct.scanCode,
                    keyboard_event_struct.vkCode,
                    KEYEVENTF_SCANCODE |KEYEVENTF_EXTENDEDKEY ,
                );
            }else{
                send_keybd_input(
                    keyboard_event_struct.scanCode,
                    keyboard_event_struct.vkCode,
                    KEYEVENTF_SCANCODE,
                );
            }
        }
    }
}



#[cfg(target_os = "linux")]
fn handle_KeyboardEvent(resp: Message) {
    println!("linux unimplemented");
    if let Ok(peripheral_event_struct) = serde_json::from_str::<LinuxEvent>(&resp.data) {
        write_to_sim_device(peripheral_event_struct);
    }
    if let Ok(keyboard_event_struct) = serde_json::from_str::<KeyboardEvent>(&resp.data) {
        let peripheral_event_struct = from_windows_keyboard_event(keyboard_event_struct);
        write_to_sim_device(peripheral_event_struct);
    }
}

#[cfg(target_os = "windows")]
fn handle_MouseEvent(resp: Message) {
    if let Ok(mouse_event_struct) = serde_json::from_str::<MouseEvent>(&resp.data) {
        let dt = chrono::prelude::Local::now();
        let milliseconds: i64 = dt.timestamp_millis();
        unsafe { MOUSEAVARAGE.push(milliseconds - mouse_event_struct.time) };
        if mouse_event_struct.flags != 0 {
            match mouse_event_struct.flags {
                MOUSEEVENTF_XDOWN | MOUSEEVENTF_XUP => send_mouse_input(
                    mouse_event_struct.flags,
                    mouse_event_struct.mouseData >> 16,
                    0,
                    0,
                ),
                MOUSEEVENTF_WHEEL | MOUSEEVENTF_HWHEEL => send_mouse_input(
                    mouse_event_struct.flags,
                    if 7864320 == mouse_event_struct.mouseData {
                        120
                    } else {
                        (120 * -1) as u32
                    },
                    0,
                    0,
                ),
                _ => send_mouse_input(mouse_event_struct.flags, 0, 0, 0),
            }
        } else {
            move_rel(mouse_event_struct.pt.0, mouse_event_struct.pt.1) //TODO
        }
    }
}
#[cfg(target_os = "linux")]
fn handle_MouseEvent(resp: Message) {
    println!("linux unimplemented");
}
fn handle_Ping(resp: Message, sender: mpsc::UnboundedSender<(Message, i32)>) {
    let dt = chrono::prelude::Local::now();
    let milliseconds: i64 = dt.timestamp_millis();
    println!(
        "Ping: {:?}",
        (milliseconds - resp.data.parse::<i64>().unwrap())
    );
    let msg = Message {
        sender: PEER_ID.to_string(),
        header: "Pong".to_string(),
        data: resp.data,
        receiver: vec![resp.sender],
    };
    if let Err(e) = sender.send((msg, 1)) {
        error!("error sending response via channel, {}", e);
    }
}

fn handle_Pong(resp: Message) {
    let dt = chrono::prelude::Local::now();
    let milliseconds: i64 = dt.timestamp_millis();
    println!(
        "Pong: {:?}",
        (milliseconds - resp.data.parse::<i64>().unwrap())
    );
}

fn handle_PublishUDP(resp: Message, sender: mpsc::UnboundedSender<(Message, i32)>) {
    println!("PublishUDP");
    handleNewUDPSocket(resp.sender.clone(), resp.data);
    let msg = Message {
        sender: PEER_ID.to_string(),
        header: "RespondwithUDP".to_string(),
        data: UDPSOCKET.local_addr().unwrap().to_string(),
        receiver: vec![resp.sender],
    };
    if let Err(e) = sender.send((msg, 1)) {
        error!("error sending response via channel, {}", e);
    }
}

fn handle_Connect(resp: Message, sender: mpsc::UnboundedSender<(Message, i32)>) {
    println!("Connect");
    if let Ok(device) = serde_json::from_str::<Device>(&resp.data) {
        unsafe {
            if TRUSTEDDEVICES
                .iter()
                .any(|mac_addr| mac_addr == &device.mac_addr)
            {
                //TODO
                //trusted device
                //auto connect
                println!("trusted...");
                println!("becoming host by default");
            }
        }
        // unsafe {
        //     DEVICENAMESMAP.insert(
        //         resp.sender.to_string(),
        //         device,
        //     );
        //     CONNECTEDTOPEERS = true;
        // };
    }
    unsafe {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "RespondConnect".to_string(),
            data: serde_json::to_string(&(DEVICENAMESMAP.get(&PEER_ID.clone().to_string()), HOST))
                .expect("can jsonify request"),
            receiver: vec![resp.sender],
        };
        if let Err(e) = sender.send((msg, 0)) {
            error!("error sending response via channel, {}", e);
        } else {
            CONNECTEDTOPEERS = true;
        }
    }
}

fn handle_RespondConnect(resp: Message) {
    if let Ok(devicebool) = serde_json::from_str::<(Device, bool)>(&resp.data) {
        let device = devicebool.0;
        let host = devicebool.1;
        unsafe {
            if host && TRUSTEDDEVICES
                .iter()
                .any(|mac_addr| mac_addr == &device.mac_addr)
            {
                //TODO
                //trusted device
                //auto connect
                // AUTOCONNECT = true;
                let mut state = AUTOCONNECT.lock().unwrap();
                *state = resp.sender.clone().as_bytes().to_vec();
                println!("trusted...");
            }
            // DEVICENAMESMAP.insert(
            //     resp.sender.to_string(),
            //     device,
            // );
            CONNECTEDTOPEERS = true;
        };
    }
}

fn handle_UpdateSets(resp: Message) {
    if let Ok(sets) = serde_json::from_str::<Vec<Set>>(&resp.data) {
        unsafe {
            SETS = sets;
        };
    }else {
        println!("BRUHHHH Sets error");
    }
}

fn handle_StartSet(resp: Message, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Ok(profile) = serde_json::from_str::<StartMessage>(&resp.data) {
        thread::spawn(move || Set::startSet(profile.set, sender));
    }
}

fn handle_Unswap(resp: Message) {
    unswap(resp.data);
}

fn handle_ConnectKey(resp: Message, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if resp.data == SUBTOPIC.lock().expect("Could not lock mutex").id() {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "ErrorConnectKey".to_string(),
            data: resp.data,
            receiver: vec![resp.sender],
        };
        if let Err(e) = sender.send((msg, 0)) {
            error!("error sending response via channel, {}", e);
        }
    }
}

fn handle_ErrorConnectKey(resp: Message, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(index) = PEER_ID.to_string().find(&resp.data) {
        println!("very very very lucky person...bool");
        handle_check_key(
            sender.clone(),
            PEER_ID.to_string().chars().count() - index - KEYLENGTH,
        )
    } else {
        println!("da fuck?");
        handle_check_key(sender.clone(), 1)
    }
}

fn handle_AttemptConnectSubTopic(resp: Message, sender: mpsc::UnboundedSender<(Message, i32)>) {
    println!("in attempt!");
    if let Ok(devicestr) = serde_json::from_str::<(Device, String)>(&resp.data) {
        let device = devicestr.0;
        let id = devicestr.1;
        println!(
            "id {:?} device: {:?} subtopicID {:?}",
            id,
            device,
            SUBTOPIC.lock().expect("Could not lock mutex").id()
        );
        unsafe {
            if id == SUBTOPIC.lock().expect("Could not lock mutex").id() && HOST {
                println!("attempting to connect...");
                let msg = Message {
                    sender: PEER_ID.to_string(),
                    header: "ConnectSubTopic".to_string(),
                    data: serde_json::to_string(&(
                        DEVICENAMESMAP.get(&PEER_ID.clone().to_string()),
                        id,
                    ))
                    .expect("can jsonify request"),
                    receiver: vec![resp.sender.clone()],
                };
                if let Err(e) = sender.send((msg, 0)) {
                    error!("error sending response via channel, {}", e);
                } else {
                    publishUDPSocket(sender.clone(), resp.sender.clone());
                    DEVICENAMESMAP.insert(resp.sender.to_string(), device);
                    // if !PUBLISHEDUDP{
                    //     println!("publushubg");
                    //     publishUDPSocket( self.response_sender.clone(), resp.sender);
                    //     PUBLISHEDUDP = true;
                    // }
                }
            }
        }
    }
}

fn handle_ConnectSubTopic(
    resp: Message,
    sender: mpsc::UnboundedSender<(Message, i32)>,
    floodsub: &mut Floodsub,
) {
    println!("ConnectSubTopic");
    if let Ok(devicestr) = serde_json::from_str::<(Device, String)>(&resp.data) {
        let device = devicestr.0;
        let id = devicestr.1;
        println!("attempting to connect...");
        let mut subtopic = SUBTOPIC.lock().expect("Could not lock mutex");
        println!("unsubscribing... {:?}", subtopic.id());
        
        floodsub.unsubscribe(subtopic.clone());
        *subtopic = Topic::new(id);
        floodsub.subscribe(subtopic.clone());

        unsafe {
            HOST = false;
            DEVICENAMESMAP.insert(resp.sender.to_string(), device);
        }
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "SuccessfulConnect".to_string(),
            data: "".to_string(),
            receiver: vec![resp.sender.clone()],
        };
        if let Err(e) = sender.send((msg, 0)) {
            error!("error sending response via channel, {}", e);
        }
        //TODO startup should be fine put private/public channels r still fucked ;-;
    }
}

fn handle_TrustedDevices(resp: Message) {
    if let Ok(trustedDevices) = serde_json::from_str::<Vec<[u8; 6]>>(&resp.data) {
        unsafe {
            TRUSTEDDEVICES = trustedDevices;
            tokio::spawn(async move {
                writeTrustedDevices(TRUSTEDDEVICES.clone()).await;
            });
        };
    };
}

fn handle_SuccessfulConnect(sender: mpsc::UnboundedSender<(Message, i32)>) {
    unsafe {
        tokio::spawn(async move {
            SETS = from_save_sets(Set::loadFromDefaultFile().await);
            updateSet(sender.clone());
        });
    }
}

fn handle_AutoConnectAttempt(resp: Message, sender: mpsc::UnboundedSender<(Message, i32)>){
    if let Ok(device) = serde_json::from_str::<Device>(&resp.data) {
    
        unsafe {
            if TRUSTEDDEVICES
                .iter()
                .any(|mac_addr| mac_addr == &device.mac_addr)
            {
                let msg = Message {
                    sender: PEER_ID.to_string(),
                    header: "AutoConnectConfirm".to_string(),
                    data: serde_json::to_string(&(DEVICENAMESMAP.get(&PEER_ID.clone().to_string()), SUBTOPIC.lock().expect("Could not lock mutex").id()))
                        .expect("can jsonify request"),
                    receiver: vec![resp.sender.clone()],
                };
                if let Err(e) = sender.send((msg, 0)) {
                    error!("error sending response via channel, {}", e);
                }else{
                    publishUDPSocket(sender.clone(), resp.sender.clone());
                    DEVICENAMESMAP.insert(resp.sender.to_string(), device);
                   
                }
            }
        }
    }
}

fn handle_AutoConnectConfirm(resp: Message, sender: mpsc::UnboundedSender<(Message, i32)>,floodsub: &mut Floodsub,){
    if let Ok(devicestr) = serde_json::from_str::<(Device,String)>(&resp.data) {
        let device = devicestr.0;
        let id = devicestr.1;
        let mut subtopic = SUBTOPIC.lock().expect("Could not lock mutex");
        println!("unsubscribing... {:?}", subtopic.id());
        floodsub.unsubscribe(subtopic.clone());
        *subtopic = Topic::new(id);
        floodsub.subscribe(subtopic.clone());
        unsafe {
            HOST = false;
            DEVICENAMESMAP.insert(resp.sender.to_string(), device);
        };
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "AutoConnectSuccess".to_string(),
            data: "".to_string(),
            receiver: vec![resp.sender.clone()],
        };
        if let Err(e) = sender.send((msg, 0)) {
            error!("error sending response via channel, {}", e);
        }
    }
}

fn handle_AutoConnectSuccess( sender: mpsc::UnboundedSender<(Message, i32)>){
    unsafe {
        tokio::spawn(async move {
            SETS = from_save_sets(Set::loadFromDefaultFile().await);
            updateSet(sender.clone());
        });
    }
}

async fn loadTrustedDevices() -> Option<Vec<[u8; 6]>> {
    let mut local_path = getlocalPath();
    local_path.push("TrustedDevices");
    if Path::new(&local_path).exists() {
        let data = fs::read_to_string(local_path)
            .await
            .expect("Unable to read file");
        let temp = serde_json::from_str::<SaveTrustedDevices>(&data).unwrap();
        return Some(temp.mac_addresses);
    }
    None
}

async fn writeTrustedDevices(mac_addresses: Vec<[u8; 6]>) {
    let mut local_path = getlocalPath();
    local_path.push("TrustedDevices");
    let data =
        serde_json::to_string(&SaveTrustedDevices { mac_addresses }).expect("can jsonify response");
    fs::write(local_path, data)
        .await
        .expect("Unable to write file");
}

fn handleNewUDPSocket(sender: String, udpAddr: String) {
    println!("Logging UDP socket");
    unsafe {
        UDPMAP.insert(sender.to_string(), udpAddr.to_string());
        println!("{:?}", UDPMAP);
    };
}

fn handle_auto_connect(receiver: String, sender: mpsc::UnboundedSender<(Message, i32)>) {
    println!("attempting to auto conenct...");
    unsafe {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "AutoConnectAttempt".to_string(),
            data: serde_json::to_string(&DEVICENAMESMAP.get(&PEER_ID.clone().to_string()))
                .expect("can jsonify request"),
            receiver: vec![receiver.clone()],
        };
        if let Err(e) = sender.send((msg, 0)) {
            error!("error sending response via channel, {}", e);
        }
    }
}
//Windows
#[cfg(target_os = "windows")]
fn addDevices() {
    PERIPHERAL_RECEIVERS
        .lock()
        .expect("Failed to unlock Mutex")
        .insert("keyboard".to_string(), vec![]);
    PERIPHERAL_RECEIVERS
        .lock()
        .expect("Failed to unlock Mutex")
        .insert("mouse".to_string(), vec![]);
}

#[cfg(target_os = "linux")]
fn addDevices() {
    let linuxdevices = get_linux_devices();
    for device in linuxdevices { 
        PERIPHERAL_RECEIVERS
        .lock()
        .expect("Failed to unlock Mutex")
        .insert(device.id.clone().to_string(), vec![]);
    }
}

//for Windows
#[cfg(target_os = "windows")]
fn init(sender: mpsc::UnboundedSender<(Message, i32)>) {
    addDevices();
    // let array = [1, 2, 3];
    // crossbeam::scope(|scope| {
    //     for i in &array {
    //         scope.spawn(move |_| {
    //             println!("element: {}", i);
    //         });
    //     }
    // });
    // crossbeam_utils::thread::scope(|scope| {

    // });
    unsafe {
        set_block_keyboard(false);
    }
    thread::spawn(move || {
        receive_keyboard_event();
    });
    let keyboard_listener = get_keyboard_recv();
    let keyboard_sender = sender.clone();
    thread::spawn(move || {
        for key_event_struct in keyboard_listener.iter() {
            handle_keyboard_event(key_event_struct, keyboard_sender.clone());
        }
    });
    unsafe {
        set_block_mouse(false);
    }
    thread::spawn(move || {
        receive_mouse_event();
    });
    let mouse_listener = get_mouse_recv();
    let mouse_sender = sender.clone();
    thread::spawn(move || {
        for mouse_event_struct in mouse_listener.iter() {
            handle_mouse_event(mouse_event_struct, mouse_sender.clone());
        }
    });
    let terminate_sender = sender.clone();
    thread::spawn(move || {
        let recv = get_exitkeys_recv();
        for _ in recv.iter() {
            handle_terminate(terminate_sender.clone());
        }
    });
    let cycle_sender = sender.clone();
    thread::spawn(move || {
        let recv = get_cyclekeys_recv();
        for _ in recv.iter() {
            handle_cycle(cycle_sender.clone());
            //cycle profiles
        }
    });
}

#[cfg(target_os = "linux")]
fn init(sender: mpsc::UnboundedSender<(Message, i32)>) {
    linux::init();
    addDevices();
    println!("devices {:?}",get_linux_devices());
    for device in get_linux_devices(){
        let recv = get_peripheral_receiver_with_id(device.id.to_string());
        let per_sender = sender.clone();//to send info to other users
        let id = device.id.to_string();
        thread::spawn(move || {
            for event in recv.iter() {
                handle_peripheral_event(id.clone(), event, per_sender.clone());
            }
        });
        let id = device.id.to_string();
        thread::spawn(move || {
            grab_peripheral_with_id(id);
        });
    }
    let terminate_sender = sender.clone();
    thread::spawn(move || {
        let recv = get_exitkeys_recv();
        for _ in recv.iter() {
            handle_terminate(terminate_sender.clone());
        }
    });
    let cycle_sender = sender.clone();
    thread::spawn(move || {
        let recv = get_cyclekeys_recv();
        for _ in recv.iter() {  
            handle_cycle(cycle_sender.clone());
            //cycle profiles
        }
    });
    reset_all_devices();
}

#[cfg(target_os = "linux")]
fn handle_peripheral_event(peripheral: String, event: LinuxEvent, sender: mpsc::UnboundedSender<(Message, i32)>){
    let temp_receivers = PERIPHERAL_RECEIVERS
        .lock()
        .expect("Failed to unlock Mutex")
        .get(&peripheral)
        .unwrap()
        .clone();
    // println!("event {:?}   {:?}", event, peripheral);

    if temp_receivers.len() != 0 {
        let mouse_buffer_clone = MOUSE_BUFFER.lock().unwrap().clone();
        let mut buff = from_linux_mouse_event(&event,mouse_buffer_clone.clone());
        if buff[0] != 0{
            // let mut buff = from_linux_mouse_event(&event);
            if buff[0] != 1 {
                println!("sending buff {:?}", buff);
            }
            let last_instant = LASTMOUSEINSTANT.lock().unwrap().elapsed();
            if last_instant.as_secs() as f64 +  last_instant.subsec_nanos() as f64 * 1e-9 > 1.0 as f64/(MOUSE_RATE.lock().unwrap().clone() as f64){
                // println!("diff: {}",last_instant.as_secs() as f64 +  last_instant.subsec_nanos() as f64 * 1e-9 - 1.0 as f64/(MOUSE_RATE.lock().unwrap().clone() as f64));
                // println!("mouserate: {:?}",MOUSE_RATE.lock().unwrap().clone());
                let mut state = MOUSE_BUFFER.lock().unwrap();
                *state = (0, 0);
                unsafe {
                    for (key, value) in &*UDPMAP {
                        if temp_receivers.iter().any(|i| i == key) {
                            UDPSOCKET.send_to(&buff, &value.clone()).unwrap();
                        }
                    }
                }
                let mut instant_state = LASTMOUSEINSTANT.lock().unwrap();
                *instant_state = Instant::now();
            }else{
                print!("limiting!");
                let mut state = MOUSE_BUFFER.lock().unwrap();
                *state = (mouse_buffer_clone.0+(buff[1] as i8) as i32, mouse_buffer_clone.1+(buff[2] as i8) as i32);
            }   
        }

        // if event.type_ == 2 {

        // }

        let mut header = "".to_string();
        if event.type_ == 1 {
            header = "KeyboardEvent".to_string();
        }
        if header != "".to_string() {
            let msg = Message {
                sender: PEER_ID.to_string(),
                header: header,
                data: serde_json::to_string(&event)
                .expect("can jsonify request"),
                receiver: temp_receivers,
            };
            if let Err(e) = sender.send((msg, 1)) {
                error!("error sending response via channel, {}", e);
            }
        }else{
            //idk add all the other events?
        }
        
    }
}

// fn respond_with_public_recipes(sender: mpsc::UnboundedSender<ListResponse>, receiver: String) {
//     tokio::spawn(async move {
//         match read_local_recipes().await {
//             Ok(recipes) => {
//                 let resp = ListResponse {
//                     mode: ListMode::ALL,
//                     receiver,
//                     data: recipes.into_iter().filter(|r| r.public).collect(),
//                 };
//                 if let Err(e) = sender.send(resp) {
//                     error!("error sending response via channel, {}", e);
//                 }
//             }
//             Err(e) => error!("error fetching local recipes to answer ALL request, {}", e),
//         }
//     });
// }

impl NetworkBehaviourEventProcess<MdnsEvent> for RecipeBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}
struct MainStruct {
    auto_connect: bool,
    auto_connect_recv: String,
    sets: Vec<Set>,
    host: bool,
    trusted_devices: Vec<[u8; 6]>,
    udpmap: HashMap<String, String>,
    device_names: HashMap<String, Device>,
    connected_to_peers: bool,
    published_udp: bool,
    current_set: String,
    subtopic: Topic,
    keyboard_receivers: Vec<String>,
    mouse_receivers: Vec<String>,
    peripheral_receivers: HashMap<String, Vec<String>>,
    terminate_threads: (Sender<bool>, Receiver<bool>),
}
#[tokio::main]
async fn main() {
    //TODO remove globals
    // let mut main_struct = MainStruct{
    //     auto_connect: false,
    //     auto_connect_recv: "".to_string(),
    //     sets: vec![],
    //     host: true,
    //     trusted_devices: vec![],
    //     udpmap: HashMap::new(),
    //     device_names: HashMap::new(),
    //     connected_to_peers: false,
    //     published_udp: false,
    //     current_set: "".to_string(),
    //     subtopic: Topic::new(""),
    //     keyboard_receivers:  vec![],
    //     mouse_receivers:  vec![],
    //     peripheral_receivers: HashMap::new(),
    //     terminate_threads: unbounded(),
    // };
    println!("socket {:?}", UDPSOCKET);
    // println!("temp {:?}",131072>>16);
    // let mouse_listener = get_mouse_recv();
    //     thread::spawn( move || {
    //         unsafe {set_block_mouse(true);}
    //         receive_mouse_event();
    // });
    // for mouse_event_struct in mouse_listener.iter() {
    //     println!("mouse_event_struct: pt: x: {:?},y: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",mouse_event_struct.pt.x,mouse_event_struct.pt.y, mouse_event_struct.mouseData,mouse_event_struct.flags,mouse_event_struct.time,mouse_event_struct.dwExtraInfo);
    // }
    pretty_env_logger::init();

    info!("Peer Id: {}", PEER_ID.clone());
    println!("Peer Id: {}", PEER_ID.clone());
    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&KEYS)
        .expect("can create auth keys");

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated()) // XX Handshake pattern, IX exists as well and IK - only XX currently provides interop with other libp2p impls
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let mut behaviour = RecipeBehaviour {
        floodsub: Floodsub::new(PEER_ID.clone()),
        mdns: TokioMdns::new().expect("can create mdns"),
        response_sender,
    };

    behaviour.floodsub.subscribe(TOPIC.clone());

    let mut swarm = SwarmBuilder::new(transp, behaviour, PEER_ID.clone())
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();
    // let mut swarm = Swarm::new(transp, behaviour, PEER_ID.clone());
    println!("maybe");
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
    .expect("swarm can be started");

    init(swarm.response_sender.clone());

    swarm
        .floodsub
        .publish(TOPIC.clone(), " ".to_string().as_bytes());
    let startup_sender = swarm.response_sender.clone();
    tokio::spawn(async move {
        unsafe {
            if let Some(trustedDevices) = loadTrustedDevices().await {
                TRUSTEDDEVICES = trustedDevices;
            }
            DEVICENAMESMAP.insert(
                PEER_ID.to_string(),
                Device {
                    name: whoami::devicename().to_string(),
                    mac_addr: mac_address::get_mac_address()
                        .unwrap()
                        .expect("got mac addr")
                        .bytes(),
                    os: whoami::platform().to_string(),
                },
            );
            loop {
                if !CONNECTEDTOPEERS {
                    println!("trying to connect....");
                    handle_start_up(startup_sender.clone());
                    thread::sleep(time::Duration::from_millis(5000)); //try to connect every 5 sec
                } else {
                    break;
                }
            }
            handle_check_key(startup_sender.clone(), 0);
            if AUTOCONNECT.lock().unwrap().len() != 0 {
                handle_auto_connect(
                    String::from_utf8(AUTOCONNECT.lock().unwrap().clone())
                        .expect("Found invalid UTF-8"),
                    startup_sender.clone(),
                );
                // handle_auto_connect(AUTOCONNECT.lock().unwrap().clone().to_string());
            }
        }
    });
    println!("here");
    //UDP socket perma listener
    
    thread::spawn(move || {
        loop {
            // println!("Receiving MOUSE MOVE!");
            let mut buf = [0; 4];
            let (amt, src) = UDPSOCKET.recv_from(&mut buf).unwrap();
            let buff = &mut buf[..amt];

            handle_udp_mouse_input(buff);
            //move_rel(mouse_event_struct.pt.0,mouse_event_struct.pt.1)
        }
    });
    println!("here2");
    loop {
        let evt = {
            tokio::select! {
                line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                event = swarm.next() => {
                    info!("Unhandled Swarm Event: {:?}", event);
                    None
                },
                response = response_rcv.recv() => Some(EventType::Response(response.expect("response exists"))),
            }
        };

        if let Some(event) = evt {
            // println!("event {:?}", event);
            //Comment
            match event {
                EventType::Response((mut resp, topic)) => {
                    //TODO
                    if resp.receiver.len() == 0 {
                        println!("public");
                        println!("message: {:?}", resp);
                        resp.receiver = get_all_users(&mut swarm).await
                    } else {
                        // println!("private")
                    }
                    if resp.header == "ConnectKey" {
                        swarm
                            .floodsub
                            .subscribe(SUBTOPIC.lock().expect("Could not lock mutex").clone());
                        println!(
                            "subbed to {:?}",
                            SUBTOPIC.lock().expect("Could not lock mutex").id()
                        )
                    }
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    if topic == 0 {
                        swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
                    } else if topic == 1 {
                        // println!("subtopic {:?}", SUBTOPIC.lock().expect("Could not lock mutex"));
                        swarm.floodsub.publish(
                            SUBTOPIC.lock().expect("Could not lock mutex").clone(),
                            json.as_bytes(),
                        );
                    } else {
                        println!("topic error")
                    }
                }
                EventType::Input(line) => match line.as_str() {
                    "ls p" => handle_list_peers(&mut swarm).await,
                    // cmd if cmd.starts_with("swap") => handle_swap(cmd,swarm.response_sender.clone()),
                    cmd if cmd.starts_with("unswap") => {
                        handle_unswap(cmd, swarm.response_sender.clone())
                    }
                    "test" => handle_test(),
                    "ping" => handle_ping(&mut swarm).await,
                    "avarage" => get_avarage(),
                    cmd if cmd.starts_with("send test") => send_test(cmd, &mut swarm).await,
                    "devices" => handle_list_devices(),
                    cmd if cmd.starts_with("newset") => {
                        handle_newset(cmd, swarm.response_sender.clone())
                    }
                    cmd if cmd.starts_with("newprofile") => {
                        handle_newprofile(cmd, swarm.response_sender.clone())
                    }
                    "loadsets" => handle_loadsets(swarm.response_sender.clone()).await,
                    "savesets" => handle_savesets(swarm.response_sender.clone()).await,
                    cmd if cmd.starts_with("editprofile") => {
                        handle_editprofile(cmd, swarm.response_sender.clone())
                    }
                    cmd if cmd.starts_with("startset") => {
                        handle_startset(cmd, swarm.response_sender.clone())
                    }
                    cmd if cmd.starts_with("startprofile") => {
                        handle_startprofile(cmd, swarm.response_sender.clone())
                    }
                    // cmd if cmd.starts_with("viewset") => handle_viewset(cmd),
                    // cmd if cmd.starts_with("viewprofile") => handle_viewprofile(cmd),
                    cmd if cmd.starts_with("editorder") => {
                        handle_editorder(cmd, swarm.response_sender.clone())
                    }
                    cmd if cmd.starts_with("setexit") => {
                        handle_setexit(cmd, swarm.response_sender.clone())
                    }
                    cmd if cmd.starts_with("setcycle") => {
                        handle_setcycle(cmd, swarm.response_sender.clone())
                    }
                    cmd if cmd.starts_with("setname") => {
                        handle_changename(cmd, swarm.response_sender.clone())
                    }
                    cmd if cmd.starts_with("viewsets") => handle_viewsets(),
                    cmd if cmd.starts_with("connectdevice") => unsafe {
                        handle_connectdevice(cmd, swarm.response_sender.clone())
                    },
                    cmd if cmd.starts_with("trustdevices") => unsafe {
                        handle_trustdevice(cmd, swarm.response_sender.clone())
                    },
                    cmd if cmd.starts_with("mouserate") => unsafe {
                        handle_mouserate(cmd, swarm.response_sender.clone())
                    },
                    "peripherals" => handle_peripherals(),
                    "subtopic" => handle_subtopic(),
                    _ => error!("unknown command"),
                },
            }
        }
    }
}
#[cfg(target_os = "windows")]
fn handle_udp_mouse_input(buff: &mut [u8]){
    if buff[3]
                == (Wrapping(buff[0] as i8) + Wrapping(buff[1] as i8) + Wrapping(buff[2] as i8)).0
                    as u8
            {
                if buff[0] == 1 {
                    // move_rel((buff[1] as i8) as i32,(buff[2]as i8) as i32);
                    send_mouse_input(
                        MOUSEEVENTF_MOVE,
                        0,
                        (buff[1] as i8) as i32,
                        (buff[2] as i8) as i32,
                    )
                } else {
                    // send_mouse_input(intToMouseFlag(buff[0]),0,0,0)
                    send_mouse_input(
                        intToMouseFlag(buff[0]),
                        0,
                        (buff[1] as i8) as i32,
                        (buff[2] as i8) as i32,
                    )
                }
            } else {
                println!("WOW THATS A SURPRISE!!!")
            }
}

#[cfg(target_os = "linux")]
fn handle_udp_mouse_input(buff: &mut [u8]){
    for event in from_windows_mouse_event(buff){
        write_to_sim_device(event);
    }
}

fn handle_subtopic() {
    println!(
        "Subbed to {:?}",
        SUBTOPIC.lock().expect("Could not lock mutex").id()
    );
}

fn handle_list_devices() {
    unsafe {
        println!("Devices: {:?}", DEVICENAMESMAP);
    }
}

fn handle_start_up(sender: mpsc::UnboundedSender<(Message, i32)>) {
    unsafe {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "Connect".to_string(),
            data: serde_json::to_string(&DEVICENAMESMAP.get(&PEER_ID.clone().to_string()))
                .expect("can jsonify request"),
            receiver: vec![],
        };
        // println!("msg {:?}",msg);
        // let json = serde_json::to_string(&msg).expect("can jsonify request");
        // swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
        sender.send((msg, 0)).expect("sent msg");
    }
}

unsafe fn handle_connectdevice(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(rest) = cmd.strip_prefix("connectdevice") {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "AttemptConnectSubTopic".to_string(),
            data: serde_json::to_string(&(
                DEVICENAMESMAP.get(&PEER_ID.clone().to_string()),
                rest.to_owned().trim(),
            ))
            .expect("can jsonify request"),
            receiver: vec![],
        };
        // println!("msg {:?}",msg);
        // let json = serde_json::to_string(&msg).expect("can jsonify request");
        // swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
        sender.send((msg, 0)).expect("sent msg");
    }
}

fn handle_check_key(sender: mpsc::UnboundedSender<(Message, i32)>, offset: usize) {
    let msg = Message {
        sender: PEER_ID.to_string(),
        header: "ConnectKey".to_string(),
        data: (&PEER_ID.to_string()[PEER_ID.to_string().to_string().len() - offset - KEYLENGTH
            ..PEER_ID.to_string().to_string().len() - offset])
            .to_owned(),
        receiver: vec![],
    };
    let mut state = SUBTOPIC.lock().expect("Could not lock mutex");
    *state = Topic::new(
        (&PEER_ID.to_string()[PEER_ID.to_string().to_string().len() - offset - KEYLENGTH
            ..PEER_ID.to_string().to_string().len() - offset])
            .to_owned(),
    );
    // behaviour.floodsub.subscribe(TOPIC.clone());
    // println!("msg {:?}",msg);
    // let json = serde_json::to_string(&msg).expect("can jsonify request");
    // swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
    sender.send((msg, 0)).expect("sent msg");
}

fn publishUDPSocket(sender: mpsc::UnboundedSender<(Message, i32)>, peer_id: String) {
    println!("Local Addr: {}", UDPSOCKET.local_addr().unwrap());
    let msg = Message {
        sender: PEER_ID.to_string(),
        header: "PublishUDP".to_string(),
        data: UDPSOCKET.local_addr().unwrap().to_string(),
        receiver: vec![peer_id],
    };
    sender.send((msg, 0)).expect("sent msg");
}
fn get_avarage() {
    unsafe {
        let mut sum1 = 0;
        for f in &KEYBOARDAVARAGE {
            sum1 += f;
        }
        println!("keyboard avarage {},", sum1 / KEYBOARDAVARAGE.len() as i64);
        let mut sum2 = 0;
        for f in &MOUSEAVARAGE {
            sum2 += f;
        }
        println!("keyboard avarage {},", sum2 / MOUSEAVARAGE.len() as i64);
    }
}

async fn handle_ping(swarm: &mut Swarm<RecipeBehaviour>) {
    let dt = chrono::prelude::Local::now();
    let milliseconds: i64 = dt.timestamp_millis();
    let msg = Message {
        sender: PEER_ID.to_string(),
        header: "Ping".to_string(),
        data: milliseconds.to_string(),
        receiver: get_all_users(swarm).await,
    };
    let json = serde_json::to_string(&msg).expect("can jsonify request");
    swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
}

fn handle_terminate(sender: mpsc::UnboundedSender<(Message, i32)>) {
    println!("unswappping......");
    println!("unswappping......");
    println!("unswappping......");
    let mut temp_string = " ".to_string();
    unsafe {
        for (k,v) in PERIPHERAL_RECEIVERS.lock().unwrap().iter() {
            temp_string += &k.clone();
            temp_string += " ";
        }
    }
    handle_unswap(&("unswap".to_string() + &temp_string), sender.clone());
}

fn reset_peripheral_receivers(){
    addDevices();
}

fn handle_cycle(sender: mpsc::UnboundedSender<(Message, i32)>) {
    let set = CURRENTSET.lock().expect("Could not lock mutex").clone();
    reset_peripheral_receivers();
    if set != "" {
        println!("cycling....");
        Set::cycleProfiles(
            CURRENTSET.lock().expect("Could not lock mutex").clone(),
            sender.clone(),
        );
    }
}


fn set_currentset(newset: String) {
    println!("here??? in currentset");
    let mut state = CURRENTSET.lock().unwrap();
    *state = newset;
    println!("here!!! in currentset");
}


#[cfg(target_os = "windows")]
fn set_keyboard_block(state: bool) {
    unsafe {
        set_block_keyboard(state);
    }
}


#[cfg(target_os = "windows")]
fn edit_peripheral_receivers(peripheral: String,recivers: Vec<String>) {
    // KEYBOARD_RECEIVERS = Mutex::new(recivers);
    // let mut state = KEYBOARD_RECEIVERS.lock().expect("Could not lock mutex");
    // *state = recivers;
    match peripheral.as_str(){
        "mouse" => {PERIPHERAL_RECEIVERS
            .lock()
            .expect("Failed to unlock Mutex")
            .insert("mouse".to_string(), recivers);
        },
        "keyboard" => {
            PERIPHERAL_RECEIVERS
            .lock()
            .expect("Failed to unlock Mutex")
            .insert("keyboard".to_string(), recivers);
        },
        _ =>()
    }
    
}

#[cfg(target_os = "linux")]
fn edit_peripheral_receivers(peripheral: String,recivers: Vec<String>) {
    PERIPHERAL_RECEIVERS
        .lock()
        .expect("Failed to unlock Mutex")
        .insert(peripheral.clone(), recivers);
}

// #[cfg(target_os = "windows")]
// fn set_mouse_recivers(recivers: Vec<String>) {
//     // let mut state = MOUSE_RECEIVERS.lock().expect("Could not lock mutex");
//     // *state = recivers;
//     PERIPHERAL_RECEIVERS
//         .lock()
//         .expect("Failed to unlock Mutex")
//         .insert("mouse".to_string(), recivers);
// }

//windows
#[cfg(target_os = "windows")]
fn set_peripheral_block(peripheral: String,blocking:bool) {
    match peripheral.as_str(){
        "mouse" => set_mouse_block(blocking),
        "keyboard" => set_keyboard_block(blocking),
        _ =>()
    }
}


#[cfg(target_os = "linux")]
fn set_peripheral_block(peripheral: String,blocking:bool) {
    println!("calling block with {}", blocking);
    let blockingsender = get_peripheral_blocker_with_id(peripheral.clone());
    blockingsender.clone().send(blocking).unwrap();
}

#[cfg(target_os = "windows")]
fn set_mouse_block(state: bool) {
    unsafe {
        set_block_mouse(state);
    }
}

#[cfg(target_os = "windows")]
fn handle_keyboard_event(
    mut key_event_struct: KeyboardEvent,
    sender: mpsc::UnboundedSender<(Message, i32)>,
) {
    let temp_receivers = PERIPHERAL_RECEIVERS
        .lock()
        .expect("Failed to unlock Mutex")
        .get(&"keyboard".to_string())
        .unwrap()
        .clone();

    if temp_receivers.len() != 0 {
        // println!("Sending keuboard event to :{:?}!", temp_receivers);
        let dt = chrono::prelude::Local::now();
        let milliseconds: i64 = dt.timestamp_millis();
        key_event_struct.time = milliseconds;
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "KeyboardEvent".to_string(),
            data: serde_json::to_string(&key_event_struct)
            .expect("can jsonify request"),
            receiver: temp_receivers,
        };
        if let Err(e) = sender.send((msg, 1)) {
            error!("error sending response via channel, {}", e);
        }
    }
    // if receivers.len() != 0 {
    //     let dt = chrono::prelude::Local::now();
    //     let milliseconds: i64 = dt.timestamp_millis();
    //     let msg = Message {
    //         sender: PEER_ID.to_string(),
    //         header: "KeyboardEvent".to_string(),
    //         data: serde_json::to_string(&KeyboardEvent {
    //             vkCode: key_event_struct.vkCode,
    //             scanCode: key_event_struct.scanCode,
    //             flags: key_event_struct.flags,
    //             time: milliseconds,
    //         })
    //         .expect("can jsonify request"),
    //         receiver: receivers,
    //     };
    //     if let Err(e) = sender.send((msg, 1)) {
    //         error!("error sending response via channel, {}", e);
    //     }
    // }
}

#[cfg(target_os = "windows")]
fn handle_mouse_event(
    mut mouse_event_struct: MouseEvent,
    sender: mpsc::UnboundedSender<(Message, i32)>,
) {
    let recivers = PERIPHERAL_RECEIVERS
        .lock()
        .expect("Failed to unlock Mutex")
        .get(&"mouse".to_string())
        .unwrap()
        .clone();
    if recivers.len() != 0 {
        let intType = mouseFlagToInt(mouse_event_struct.flags);
        if mouse_event_struct.flags == 0 {
            let mouse_buffer_clone = MOUSE_BUFFER.lock().unwrap().clone();
            let mut buff: [u8; 4] = [0; 4];
            buff[0] = 1 as u8;
            buff[1] = (mouse_event_struct.pt.0 as i8) as u8;
            buff[2] = (mouse_event_struct.pt.1 as i8) as u8;
            buff[3] = (Wrapping(buff[0] as i8) + Wrapping(buff[1] as i8) + Wrapping(buff[2] as i8))
                .0 as u8;

            let last_instant = LASTMOUSEINSTANT.lock().unwrap().elapsed();
            let mouse_rate = MOUSE_RATE.lock().unwrap().clone() as f64;
            if last_instant.as_secs() as f64 +  last_instant.subsec_nanos() as f64 * 1e-9 > 1.0 as f64/mouse_rate{
                println!("diff: {}",last_instant.as_secs() as f64 +  last_instant.subsec_nanos() as f64 * 1e-9 - 1.0 as f64/mouse_rate);
                println!("mouserate: {:?}",mouse_rate);
                let mut state = MOUSE_BUFFER.lock().unwrap();
                *state = (0, 0);
                unsafe {
                    for (key, value) in &*UDPMAP {
                        if recivers.iter().any(|i| i == key) {
                            UDPSOCKET.send_to(&buff, &value.clone()).unwrap();
                        }
                    }
                }
                let mut instant_state = LASTMOUSEINSTANT.lock().unwrap();
                *instant_state = Instant::now();
            }else{
                let mut state = MOUSE_BUFFER.lock().unwrap();
                *state = (mouse_buffer_clone.0+(buff[1] as i8) as i32, mouse_buffer_clone.1+(buff[2] as i8) as i32);
            }
            
        } else if intType != 0 {
            let mut buff: [u8; 4] = [0; 4];
            buff[0] = intType;
            buff[1] = 0;
            buff[2] = 0;
            buff[3] = intType;
            unsafe {
                for (key, value) in &*UDPMAP {
                    // println!("Sending mouse btn");
                    if recivers.iter().any(|i| i == key) {
                        UDPSOCKET.send_to(&buff, &value.clone()).unwrap();
                    }
                }
            }
        } else {
            let dt = chrono::prelude::Local::now();
            let milliseconds: i64 = dt.timestamp_millis();
            mouse_event_struct.time = milliseconds;
            let msg = Message {
                sender: PEER_ID.to_string(),
                header: "MouseEvent".to_string(),
                data: serde_json::to_string(&mouse_event_struct).expect("can jsonify request"),
                receiver: recivers,
            };
            println!("Sending mouse sidebtn");
            if let Err(e) = sender.send((msg, 0)) {
                error!("error sending response via channel, {}", e);
            }
        }
    }
}


#[cfg(target_os = "windows")]
fn mouseFlagToInt(flag: DWORD) -> u8 {
    return match flag {
        MOUSEEVENTF_LEFTDOWN => 2,
        MOUSEEVENTF_RIGHTDOWN => 3,
        MOUSEEVENTF_MIDDLEDOWN => 4,
        MOUSEEVENTF_LEFTUP => 5,
        MOUSEEVENTF_RIGHTUP => 6,
        MOUSEEVENTF_MIDDLEUP => 7,
        _ => 0,
    };
}
#[cfg(target_os = "windows")]
fn intToMouseFlag(flag: u8) -> DWORD {
    return match flag {
        2 => MOUSEEVENTF_LEFTDOWN,
        3 => MOUSEEVENTF_RIGHTDOWN,
        4 => MOUSEEVENTF_MIDDLEDOWN,
        5 => MOUSEEVENTF_LEFTUP,
        6 => MOUSEEVENTF_RIGHTUP,
        7 => MOUSEEVENTF_MIDDLEUP,
        _ => 0,
    };
}
#[cfg(target_os = "windows")]
fn unswap(rest: String) {
    let elements: Vec<&str> = rest.split(" ").collect();
    for f in &elements {
        match f {
            &"mouse" => {
                set_peripheral_block("mouse".to_string(), false);
                // set_mouse_recivers(vec![]);
                edit_peripheral_receivers("mouse".to_string(), vec![]);
            }
            &"keyboard" => {
                set_peripheral_block("keyboard".to_string(), false);
                // set_keyboard_recivers(vec![]);
                edit_peripheral_receivers("keyboard".to_string(), vec![]);
            }
            invalid => {
                println!("Invalid item to swap: {}", invalid)
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn unswap(rest: String) {
    let elements: Vec<&str> = rest.split(" ").collect();
    for f in &elements {
        match f {
            &"" => (),
            &" " => (),
            id => {
                set_peripheral_block(id.clone().to_string(), false);
                // set_keyboard_recivers(vec![]);
                edit_peripheral_receivers(id.clone().to_string(), vec![]);
            }
        }
    }
}

fn handle_unswap(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(rest) = cmd.strip_prefix("unswap") {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "Unswap".to_string(),
            data: rest.to_owned(),
            receiver: vec![],
        };
        if let Err(e) = sender.send((msg, 0)) {
            error!("error sending response via channel, {}", e);
        } else {
            unswap(rest.to_owned());
        }
    }
}

// fn handle_swap(cmd: &str ,sender: mpsc::UnboundedSender<(Message, i32)>) {
//     // thread::spawn(move || {
//     //     receive_keyboard_event();
//     // });
//     // receive_mouse_event();
//     // let msg = Message {
//     //     header: "KeyboardEvent".to_string(),
//     //     data: "lolol".to_string(),
//     //     receiver: vec![],
//     // };
//     // if let Err(e) = sender.clone().send((msg,0)) {
//     //     error!("error sending response via channel, {}", e);
//     // }
//     if let Some(rest) = cmd.strip_prefix("swap") {
//         let elements: Vec<&str> = rest.split(" ").collect();
//         for f in &elements {
//            match f {
//             &"mouse" => {
//                 swap_mouse(sender.clone())
//             },
//             &"keyboard" => {
//                 swap_keyboard(sender.clone())
//             },
//             invalid =>{
//                 println!("Invalid item to swap: {}", invalid)
//             }
//            }
//         }
//     }
// }
async fn handle_list_peers(swarm: &mut Swarm<RecipeBehaviour>) {
    let nodes = swarm.mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().for_each(|p| info!("{}", p));
    unique_peers.iter().for_each(|p| println!("{}", p));
}

async fn get_all_users(swarm: &mut Swarm<RecipeBehaviour>) -> Vec<String> {
    let nodes = swarm.mdns.discovered_nodes();
    let mut array: Vec<String> = Vec::new();
    for peer in nodes {
        if !array.contains(&peer.to_string()) {
            array.push(peer.to_string());
        }
    }
    array
}

async fn send_test(cmd: &str, swarm: &mut Swarm<RecipeBehaviour>) {
    let sender = swarm.response_sender.clone();
    if let Some(rest) = cmd.strip_prefix("send test") {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "Test".to_string(),
            data: rest.to_string(),
            receiver: get_all_users(swarm).await,
        };
        let json = serde_json::to_string(&msg).expect("can jsonify request");
        swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
    }
}

fn handle_newset(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(rest) = cmd.strip_prefix("newset") {
        if Set::new(rest.trim().to_owned(), sender.clone()) {
            println!("success!");
            return;
        }
    }
    println!("failure to create new set!");
}
fn handle_newprofile(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(rest) = cmd.strip_prefix("newprofile") {
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        if Set::newProfile(
            elements[0].trim().to_owned(),
            elements[1].trim().to_owned(),
            sender.clone(),
        ) {
            println!("success!");
            return;
        }
    }
    println!("failure to create new set!");
}

fn handle_editprofile(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(rest) = cmd.strip_prefix("editprofile") {
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        let mut receivers = vec![];
        let mut sender_id = elements[3].trim().to_owned();
        match searchDeviceName(elements[3].trim().to_owned()).as_str() {
            "" => (),
            a => sender_id = a.to_owned(),
        };
        for i in 4..elements.len() {
            match searchDeviceName(elements[i].trim().to_owned()).as_str() {
                "" => receivers.push(elements[i].trim().to_owned()),
                a => receivers.push(a.to_owned()),
            };
        }
        println!("{:?}  {:?}  {:?}  {:?}  {:?}  ",elements[0].trim().to_owned(),
        elements[1].trim().to_owned(),
        elements[2].trim().to_owned(),
        sender_id,
        receivers);
        Set::editProfile(
            elements[0].trim().to_owned(),
            elements[1].trim().to_owned(),
            elements[2].trim().to_owned(),
            sender_id,
            receivers,
            sender.clone(),
        )
    }
}

fn handle_changename(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(rest) = cmd.strip_prefix("setname") {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "ChangeName".to_string(),
            data: serde_json::to_string(&Device {
                name: rest.trim().to_owned(),
                mac_addr: mac_address::get_mac_address()
                    .unwrap()
                    .expect("got mac addr")
                    .bytes(),
                os: whoami::platform().to_string(),
            })
            .expect("can jsonify request"),
            receiver: vec![],
        };
        if let Err(e) = sender.send((msg, 0)) {
            error!("error sending response via channel, {}", e);
        } else {
            unsafe {
                DEVICENAMESMAP.insert(
                    PEER_ID.clone().to_string(),
                    Device {
                        name: rest.trim().to_owned(),
                        mac_addr: mac_address::get_mac_address()
                            .unwrap()
                            .expect("got mac addr")
                            .bytes(),
                        os: whoami::platform().to_string(),
                    },
                );
            };
        }
    }
}

fn handle_startset(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    updateSet(sender.clone());
    if let Some(rest) = cmd.strip_prefix("startset") {
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "StartSet".to_string(),
            data: serde_json::to_string(&StartMessage {
                set: elements[0].to_owned(),
                profile: Set::getfirstprofile(elements[0].to_owned()),
            })
            .expect("can jsonify request"),
            receiver: vec![],
        };
        if let Err(e) = sender.clone().send((msg, 0)) {
            error!("error sending response via channel, {}", e);
        } else {
            println!("starting local????");
            let set_id = elements[0].to_owned();
            thread::spawn(move || {
                Set::startSet(set_id, sender.clone());
            });
        }
    }
}

fn handle_startprofile(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    updateSet(sender.clone());
    if let Some(rest) = cmd.strip_prefix("startprofile") {
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "StartSet".to_string(),
            data: serde_json::to_string(&StartMessage {
                set: elements[0].to_owned(),
                profile: elements[1].to_owned(),
            })
            .expect("can jsonify request"),
            receiver: vec![],
        };
        if let Err(e) = sender.send((msg, 0)) {
            error!("error sending response via channel, {}", e);
        } else {
            //TODO i think this is blocking :/
            Set::startProfile(
                elements[0].to_owned(),
                elements[1].to_owned(),
                sender.clone(),
            )
        }
    }
}

async fn handle_loadsets(sender: mpsc::UnboundedSender<(Message, i32)>) {
    let loadedset = from_save_sets(Set::loadFromDefaultFile().await);
    println!("loadedset: {:?}", loadedset);
    if loadedset.len() != 0 {
        unsafe { SETS = loadedset }
        updateSet(sender.clone());
    }
}

async fn handle_savesets(sender: mpsc::UnboundedSender<(Message, i32)>) {
    Set::saveToDefaultFile().await;
    println!("saved!");
}

fn handle_viewsets() {
    unsafe {
        println!("SETS {:?}", SETS);
    }
}
fn handle_setexit(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(rest) = cmd.strip_prefix("setexit") {
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        Set::setexit(
            elements[0].to_owned(),
            elements[1..]
                .to_vec()
                .iter()
                .map(|x| (*x).to_owned().parse::<u32>().unwrap())
                .collect(),
            sender.clone(),
        );
    }
}
fn handle_setcycle(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(rest) = cmd.strip_prefix("setcycle") {
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        Set::setcycle(
            elements[0].to_owned(),
            elements[1..]
                .to_vec()
                .iter()
                .map(|x| (*x).to_owned().parse::<u32>().unwrap())
                .collect(),
            sender.clone(),
        );
    }
}
fn handle_editorder(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    //TODO
    if let Some(rest) = cmd.strip_prefix("editorder") {
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        Set::editorder(
            elements[0].to_owned(),
            elements[1..]
                .to_vec()
                .iter()
                .map(|x| (*x).to_owned())
                .collect(),
            sender.clone(),
        );
    }
}

fn handle_trustdevice(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(rest) = cmd.strip_prefix("trustdevices") {
        let mut elements: Vec<&str> = rest.trim().split(",").collect();
        println!("elements {:?}", elements);
        unsafe {
            let name = DEVICENAMESMAP
                .get(&PEER_ID.clone().to_string())
                .unwrap()
                .name
                .clone();
            elements.push(&name);

            for devicename in &elements {
                if searchDeviceName(devicename.trim().to_string()) != "".to_string() {
                    let device = DEVICENAMESMAP.get(&searchDeviceName(devicename.trim().to_string())).unwrap();
                    if !TRUSTEDDEVICES.contains(&device.mac_addr) {
                        TRUSTEDDEVICES.push(device.mac_addr.clone());
                    }
                }else{
                    println!("bruh.... {:?}  {:?}",devicename, devicename.trim());
                }
                
            }
            let msg = Message {
                sender: PEER_ID.to_string(),
                header: "TrustedDevices".to_string(),
                data: serde_json::to_string(&TRUSTEDDEVICES).expect("can jsonify request"),
                receiver: vec![],
            };
            if let Err(e) = sender.send((msg, 1)) {
                error!("error sending response via channel, {}", e);
            } else {
                tokio::spawn(async move {
                    writeTrustedDevices(TRUSTEDDEVICES.clone()).await;
                });
            }
        }
    }
}

fn handle_test(){
    println!("PERIPHERAL_RECEIVERS {:?}", PERIPHERAL_RECEIVERS.lock().expect("Could not lock mutex"));
}

#[cfg(target_os = "windows")]
fn handle_peripherals(){
    println!("mouse");
    println!("keyboard");
}


#[cfg(target_os = "linux")]
fn handle_peripherals(){
    for device in get_linux_devices(){
        println!("id: {:?} name:{:?}", device.id, device.name);
    }
}

fn handle_mouserate(cmd: &str, sender: mpsc::UnboundedSender<(Message, i32)>) {
    if let Some(rest) = cmd.strip_prefix("mouserate") {
        let newrate = rest.trim().to_owned().parse::<i32>().unwrap();
        let mut state = MOUSE_RATE.lock().unwrap();
        *state = newrate;
    }
}
