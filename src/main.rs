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
    peripheral_receivers: HashMap<String,HashMap<(String,bool), Vec<(String,bool)>>>,
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
        reset_ctrl_shift()
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
        let mut new_receivers = vec![];
        for receiver in receivers{
            new_receivers.push((receiver.clone(),true))
        }
        temp_hashmap.insert((sender,true), new_receivers);
        
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
            if let Some(val) = targets.get(&(PEER_ID.to_string(),true)) {
                println!("inside {:?}",val);
                if val.clone().iter().any(|i| (*i.0) == PEER_ID.to_string()) {
                    set_peripheral_block(peripheral.clone(),false);
                } else {
                    set_peripheral_block(peripheral.clone(),true);
                }
                let mut receivers = vec![];
                for receiver in val{
                    if receiver.1 == true{
                        receivers.push(receiver.0.clone())
                    }
                }
                PERIPHERAL_RECEIVERS
                    .lock()
                    .expect("Failed to unlock Mutex")
                    .insert(peripheral.clone(), receivers.clone());
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
                        if key2.1 == true{
                            temp_hashmap.insert(
                                from_mac_addr_to_string(DEVICENAMESMAP.get(&key2.0).expect(&key2.0).mac_addr.clone().to_vec()),
                                value2
                                    .clone()
                                    .into_iter()
                                    .map(|x| {
                                            if x.1 == true{
                                                from_mac_addr_to_string(DEVICENAMESMAP.get(&x.0.clone()).expect(&x.0).mac_addr.clone().to_vec())
                                            }else{
                                                x.0
                                            }   
                                        }
                                    )
                                    .collect(),
                            );
                        }else{
                            temp_hashmap.insert(
                                key2.0.clone(),
                                value2
                                    .clone()
                                    .into_iter()
                                    .map(|x| {
                                            if x.1 == true{
                                                from_mac_addr_to_string(DEVICENAMESMAP.get(&x.0.clone()).expect(&x.0).mac_addr.clone().to_vec())
                                            }else{
                                                x.0
                                            }   
                                        }
                                    )
                                    .collect(),
                            );
                        }
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
                                (peer_id,true),
                                value2
                                    .clone()
                                    .into_iter()
                                    .map(|x| {
                                        let temp = searchDeviceMacAddress(from_string_to_mac_addr(x.clone()));
                                        if temp == "".to_string(){
                                            (x.clone(),false)
                                        }else{
                                            (temp,true)
                                        }
                                        
                                    })
                                    .collect(),
                            );
                        }else{
                            temp_hashmap.insert(
                                (key2.clone(),false),
                                value2
                                    .clone()
                                    .into_iter()
                                    .map(|x| {
                                        let temp = searchDeviceMacAddress(from_string_to_mac_addr(x.clone()));
                                        if temp == "".to_string(){
                                            (x.clone(),false)
                                        }else{
                                            (temp,true)
                                        }
                                    })
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
    if let Ok(device) = serde... (53 KB left)
