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
use std::{thread, time};
use std::net::{SocketAddr, UdpSocket};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::{fs, io::AsyncBufReadExt, sync::mpsc};
use std::collections::HashMap;
use std::num::Wrapping;
use local_ipaddress;
use mac_address;
use std::env;
use std::io;
use std::path::PathBuf;

use chrono;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("Capstone"));
static mut KEYBOARDAVARAGE: Vec<i64> = vec![];
static mut MOUSEAVARAGE: Vec<i64> = vec![];
static UDPSOCKET:Lazy<UdpSocket> =  Lazy::new(|| UdpSocket::bind(local_ipaddress::get().unwrap()+":0").expect("couldn't bind to address"));
static mut UDPMAP:Lazy<HashMap<String, String>> = Lazy::new(|| HashMap::new());
static mut DEVICENAMESMAP:Lazy<HashMap<String, Device>> = Lazy::new(|| HashMap::new());
static mut PUBLISHEDUDP:bool = false;
static mut CONNECTEDTOPEERS:bool = false;
// static mut CURRENTSET:Set;
// static mut KEYBOARDDESTINATION: Vec<String> = vec![];
// static mut MOUSEDESTINATION: Vec<String> = vec![];
static mut SETS: Vec<Set> = vec![];

lazy_static! {
    static ref TERMINATETHREADS: (Mutex<Sender<bool>>,Mutex<Receiver<bool>>) = {
        let (send, recv) = unbounded();
        (Mutex::new(send), Mutex::new(recv))
    };
    static ref KEYBOARD_RECIEVERS: Mutex<Vec<String>> = Mutex::new(vec![]);
    static ref MOUSE_RECIEVERS: Mutex<Vec<String>> = Mutex::new(vec![]);
    static ref CURRENTSET: Mutex<String> = Mutex::new("".into());
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use crate::windows::*;

fn searchDeviceName(device_name: String) -> String{
    unsafe{
        for (peer_id, device) in &*DEVICENAMESMAP {
            if device.name == device_name{
                return peer_id.clone().to_owned();
            }
        }
        return "".to_string()
    }
    
}

fn getlocalPath() -> PathBuf {
    let mut dir = env::current_exe().unwrap();
    dir.pop();
    println!("{}", dir.display());
    dir
}

#[derive(Debug, Serialize, Deserialize)]
struct Device{//struct with name, mac addr, and OS maybe more
    name: String,
    mac_addr: [u8; 6],
    os: String,
}

impl Device {
    fn editName(self, peer_id:String, new_name: String,sender: mpsc::UnboundedSender<Message>) {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "EditDeviceName".to_string(),
            data: peer_id+" "+&new_name,
            receiver: vec![],
        };
        if let Err(e) = sender.send(msg) {
            error!("error sending response via channel, {}", e);
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Profile {
    mouse_target:   HashMap<String, Vec<String>>,
    keyboard_target: HashMap<String, Vec<String>>,
    id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Set {
    profiles: Vec<Profile>,
    id:String,
    exitkeys: Vec<u32>,
    cyclekeys:Vec<u32>,
    profile_order: Vec<String>
}

impl Set {
    fn new(id:String, sender:  mpsc::UnboundedSender<Message>) -> bool {
        if Set::findSet(id.clone()) != -1 {
            return false
        }
        let set = Set{profiles: vec![],id, exitkeys: vec![29,42,34],cyclekeys: vec![29,42,35],profile_order: vec![]};
        unsafe {SETS.push(set);};
        updateSet(sender.clone());
        return true
        // &PROFILES[PROFILES.len()-1]
    }
    fn newProfile(set_id:String, profile_id:String, sender:  mpsc::UnboundedSender<Message>) -> bool {
        // self.profiles.rooms.iter().find(|&x| x.id == id);
        println!("set_id: {:?}, profile_id: {:?}", set_id,profile_id);
        unsafe {
            // let mut set = SETS[Set::findSet(set_id) as usize];
            let index = Set::findSet(set_id.clone());
            if index == -1 {
                return false
            }
            if SETS[index as usize].findProfile(profile_id.clone()) != -1 {
                return false
            }
            let profile = Profile{mouse_target: HashMap::new(),keyboard_target: HashMap::new(),id:profile_id.clone()};
            SETS[index as usize].profiles.push(profile);
            SETS[index as usize].profile_order.push(profile_id);
        };
        updateSet(sender.clone());
        true
        // &PROFILES[PROFILES.len()-1]
    }
    fn view(&self) -> Vec<Profile>{
        return self.profiles.clone()
    }
    fn findSet(id:String) -> i32 {
        unsafe {
            if let Some(pos) = SETS.iter().position(|x| x.id == id){
                return pos as i32
            }
            return -1
        }
    }
    fn findProfile(&mut self, profile_id:String) -> i32 {
        if let Some(pos) = self.profiles.iter().position(|x| x.id == profile_id){
            return pos as i32
        }
        -1
    }
    fn removeProfile(&mut self, profile_id:String, sender:  mpsc::UnboundedSender<Message>){
        self.profiles.retain(|x| x.id != profile_id);
        updateSet(sender.clone());
    }
    fn getProfile(&mut self, profile_id:String) -> Option<&Profile>{
        let temp = self.profiles.iter().find(|&x| x.id == profile_id);
        temp
    }
    fn delete(&mut self, sender:  mpsc::UnboundedSender<Message>){
        unsafe {SETS.retain(|x| x.id != self.id);};
        updateSet(sender.clone());
    }
    fn editProfile(set_id:String, profile_id:String, peripheral: String, sender_id:String, receivers: Vec<String>, sender:  mpsc::UnboundedSender<Message>){
        unsafe{
            if let Some(pos) =   SETS[Set::findSet(set_id.clone()) as usize].profiles.iter().position(|profile| profile.id == profile_id){
                let z = &mut SETS[Set::findSet(set_id) as usize].profiles[pos];
                z.edit(peripheral, sender_id, receivers);
            } 
        }
        updateSet(sender.clone());
        
    }
    async fn loadFromDefaultFile() -> Vec<Set> {
        let mut local_path = getlocalPath();
        local_path.push("sets.txt");
        if local_path.exists() {
            let data = fs::read_to_string(local_path).await.expect("Unable to read file");
            let temp = serde_json::from_str::<Vec<Set>>(&data).unwrap();
            return temp
        }
        println!("couldnt open");
        return vec![]
        
    }
    async fn saveToDefaultFile() {
        unsafe {
            let mut local_path = getlocalPath();
            local_path.push("sets.txt");
            println!("final path {:?}", local_path);
            let data = serde_json::to_string(&SETS).expect("can jsonify response");
            fs::write(local_path, data).await.expect("Unable to write file");
        }   
    }
    fn cycleProfiles(set_id:String, sender:mpsc::UnboundedSender<Message>){
        unsafe{
            SETS[Set::findSet(set_id.clone()) as usize].profile_order.rotate_left(1);
            println!("cycled to {:?}", SETS[Set::findSet(set_id.clone()) as usize].profile_order[0].clone());
            
            let msg = Message {
                sender: PEER_ID.to_string(),
                header: "StartSet".to_string(),
                data: serde_json::to_string(&StartMessage{set:set_id.clone(), profile: SETS[Set::findSet(set_id.clone()) as usize].profile_order[0].clone()}).expect("can jsonify request"),
                receiver: vec![]
            };
            if let Err(e) = sender.send(msg) {
                error!("error sending response via channel, {}", e);
            }else{
                thread::spawn( move || {
                    Set::startSet(set_id, sender.clone())
                });
                
            }
        }
        
    }
    fn startProfile(set_id:String,profile_id:String, sender:mpsc::UnboundedSender<Message>){
        unsafe{
            while SETS[Set::findSet(set_id.clone()) as usize].profile_order[0] != profile_id{
                SETS[Set::findSet(set_id.clone()) as usize].profile_order.rotate_left(1);
            }
            
        };
        thread::spawn( move || {Set::startSet(set_id,sender);});
        
    }
    fn startSet(set_id:String, sender:mpsc::UnboundedSender<Message>){//TODO here
        println!("in startset");
        println!("someone explain... {:?}",set_id);
        set_currentset(set_id.clone());
        println!("set1");
        unsafe {
            println!("set2");
            setexitkeys(SETS[Set::findSet(set_id.clone()) as usize].exitkeys.clone());
            println!("set3");
            setcyclekeys(SETS[Set::findSet(set_id.clone()) as usize].cyclekeys.clone());
            println!("set4");
            SETS[Set::findSet(set_id.clone()) as usize].getProfile(SETS[Set::findSet(set_id) as usize].profile_order[0].clone()).unwrap().load(sender.clone());
        };
        println!("finished startset");
        // broadcastSet(set_id.clone(), sender.clone());
        // SETS[Set::findSet(set_id) as usize].profiles[0].load();
    }
    fn setexit(set_id:String, keys:Vec<u32>, sender:mpsc::UnboundedSender<Message>){
        unsafe{
            SETS[Set::findSet(set_id.clone()) as usize].exitkeys = keys;
        }
        updateSet(sender.clone());
    }
    fn setcycle(set_id:String, keys:Vec<u32>, sender:mpsc::UnboundedSender<Message>){
        unsafe{
            SETS[Set::findSet(set_id.clone()) as usize].exitkeys = keys;
        }
        updateSet(sender.clone());
    }
    fn editorder(set_id:String, order:Vec<String>, sender:mpsc::UnboundedSender<Message>){
        unsafe{
            SETS[Set::findSet(set_id.clone()) as usize].profile_order = order;
        }
    }
    fn getfirstprofile(set_id:String) -> String{
        unsafe{
            return SETS[Set::findSet(set_id.clone()) as usize].profile_order[0].clone();
        }
    }
}

fn updateSet(sender:mpsc::UnboundedSender<Message>){
    unsafe {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "UpdateSets".to_string(),
            data: serde_json::to_string(&SETS).expect("can jsonify request"),                   
            receiver: vec![]
        };
        if let Err(e) = sender.clone().send(msg) {
            error!("error sending response via channel, {}", e);
        }
    }
}

fn broadcastSet(set_id:String, sender:  mpsc::UnboundedSender<Message>){
    unsafe {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "Set".to_string(),
            data: serde_json::to_string(&SETS[Set::findSet(set_id) as usize]).expect("can jsonify request"),                   
            receiver: vec![]
        };
        sender.send(msg).expect("sent msg");
    }
}

impl Profile {

    fn edit(&mut self, peripheral: String, sender:String, receivers: Vec<String>){
        if peripheral == "mouse"{
            self.mouse_target.insert(sender, receivers);
        }else if peripheral == "keyboard"{
            self.keyboard_target.insert(sender, receivers);
        }else{
            return
        }
        // hash_map[sender] = receivers;
    
        // ig make nicknames
    }//TODO 
    fn load(&self, sender:  mpsc::UnboundedSender<Message> ){
        println!("loading...");
        if let Some(val) = self.mouse_target.get(&PEER_ID.to_string()){
            println!("SWAPPING MOUSE {:?}",val.clone());
            // swap_mouse(sender.clone(), val.clone());
            if val.clone().iter().any(|i| (*i) == PEER_ID.to_string()) {
                set_mouse_block(false);
            }else{
                set_mouse_block(true);
            }
            set_mouse_recivers(val.clone());
        }else{
            println!("NOTSWAPPING MOUSE");
            // swap_mouse(sender.clone(), vec![PEER_ID.to_string()]);
            set_mouse_recivers(vec![]);
            set_mouse_block(false);
        }
        if let Some(val) = self.keyboard_target.get(&PEER_ID.to_string()){
            println!("SWAPPING Keyboard {:?}",val.clone());
            // swap_keyboard(sender.clone(), val.clone());
            if val.clone().iter().any(|i| (*i) == PEER_ID.to_string()) {
                set_keyboard_block(false);
            }else{
                set_keyboard_block(true);
            }
            set_keyboard_recivers(val.clone());
        }else{
            println!("NOTSWAPPING KEYBOARD");
            // swap_keyboard(sender.clone(), vec![PEER_ID.to_string()]);//TODO test swapping and closing....
            set_keyboard_block(false);
            set_keyboard_recivers(vec![]);
        }
        // let mouse_recievers = self.mouse_target.get(&PEER_ID.to_string()).unwrap().clone();
        
        // let keyboard_recievers = self.keyboard_target.get(&PEER_ID.to_string()).unwrap().clone();
        // swap_keyboard(sender.clone(),keyboard_recievers);
    }
    fn save(){

    }
    fn loadFromFile(){
        
    }
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
    Response(Message),
    Input(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyboardEvent {
    vkCode: u32,
    scanCode: u32,
    flags: u32,
    time: i64,
}


#[derive(NetworkBehaviour)]
struct RecipeBehaviour {
    floodsub: Floodsub,
    mdns: TokioMdns,
    #[behaviour(ignore)]
    response_sender: mpsc::UnboundedSender<Message>,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for RecipeBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(msg) => {
                // println!("msg!");
                if let Ok(resp) = serde_json::from_slice::<Message>(&msg.data) {
                    if resp.receiver.contains(&PEER_ID.to_string()){
                        if resp.header == "Test".to_string() {
                            println!("perfect. Data: {:?} ",resp.data);
                        } else if resp.header == "KeyboardEvent".to_string() {
                            
                            if let Ok(keyboard_event_struct) = serde_json::from_str::<KeyboardEvent>(&resp.data) {  
                                let dt = chrono::prelude::Local::now();
                                let milliseconds: i64= dt.timestamp_millis();
                                unsafe {KEYBOARDAVARAGE.push(milliseconds - keyboard_event_struct.time)};
                                if  keyboard_event_struct.vkCode == 66 {
                                    println!("AAAAA :D")
                                }
                                if keyboard_event_struct.flags == 128{
                                    send_keybd_input(keyboard_event_struct.scanCode,keyboard_event_struct.vkCode,KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);                      
                                }else if keyboard_event_struct.flags == 0{
                                    send_keybd_input(keyboard_event_struct.scanCode,keyboard_event_struct.vkCode,KEYEVENTF_SCANCODE );                      
                                }else if keyboard_event_struct.flags == 129{
                                    send_keybd_input(0,keyboard_event_struct.vkCode,KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);                      
                                }else if keyboard_event_struct.flags == 1{
                                    send_keybd_input(0,keyboard_event_struct.vkCode,KEYEVENTF_UNICODE );                      
                                }else if keyboard_event_struct.flags == 32{//make sure
                                    send_keybd_input(0,keyboard_event_struct.vkCode,KEYEVENTF_UNICODE );                      
                                }else if (keyboard_event_struct.flags >> 5) % 2 ==  1{//make sure
                                    send_keybd_input(0,keyboard_event_struct.vkCode,KEYEVENTF_UNICODE );                      
                                }else if (keyboard_event_struct.flags >> 7) % 2 ==  1{//make sure
                                    send_keybd_input(0,keyboard_event_struct.vkCode,KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);                      
                                }else{
                                    println!("SOMETGHING DIFFERENT  {:?}",keyboard_event_struct.flags);
                                    send_keybd_input(0,keyboard_event_struct.vkCode,KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);                      
                                }
                                //unhandled events : 160,161,33 (33 and 161 r together idfk 160)
                            }
                        }else if resp.header == "MouseEvent".to_string() {
                            
                            if let Ok(mouse_event_struct) = serde_json::from_str::<MouseEvent>(&resp.data) {  
                                let dt = chrono::prelude::Local::now();
                                let milliseconds: i64= dt.timestamp_millis();
                                unsafe {MOUSEAVARAGE.push(milliseconds - mouse_event_struct.time)};
                               
                                if mouse_event_struct.flags != 0 {
                                    match mouse_event_struct.flags{
                                        MOUSEEVENTF_XDOWN|MOUSEEVENTF_XUP => send_mouse_input(mouse_event_struct.flags,mouse_event_struct.mouseData>>16,0,0),
                                        MOUSEEVENTF_WHEEL|MOUSEEVENTF_HWHEEL =>send_mouse_input(mouse_event_struct.flags,if 7864320 == mouse_event_struct.mouseData {120} else {(120*-1) as u32},0,0),
                                        _=> send_mouse_input(mouse_event_struct.flags,0,0,0)
                                    }
                                    
                                }else{
                                    move_rel(mouse_event_struct.pt.0,mouse_event_struct.pt.1)  //TODO
                                }
                                 
                                //unhandled events : 160,161,33 (33 and 161 r together idfk 160)
                            }
                        }else if resp.header == "Ping".to_string() {
                            let dt = chrono::prelude::Local::now();
                            let milliseconds: i64= dt.timestamp_millis();
                            println!("Ping: {:?}", (milliseconds-resp.data.parse::<i64>().unwrap()));
                            let msg = Message {
                                sender: PEER_ID.to_string(),
                                header: "Pong".to_string(),
                                data: resp.data,
                                receiver: vec![resp.sender],
                            };
                            if let Err(e) = self.response_sender.send(msg) {
                                error!("error sending response via channel, {}", e);
                            }
                        }else if resp.header == "Pong".to_string() {
                            let dt = chrono::prelude::Local::now();
                            let milliseconds: i64= dt.timestamp_millis();
                            println!("Pong: {:?}", (milliseconds-resp.data.parse::<i64>().unwrap()));
                        } else if resp.header == "PublishUDP"{
                            println!("PublishUDP");
                            handleNewUDPSocket(resp.sender.clone(), resp.data);
                            let msg = Message {
                            sender: PEER_ID.to_string(),
                            header: "RespondwithUDP".to_string(),
                            data: UDPSOCKET.local_addr().unwrap().to_string(),
                            receiver: vec![resp.sender]
                            };
                            if let Err(e) = self.response_sender.send(msg) {
                                error!("error sending response via channel, {}", e);
                            }          
                        }else if resp.header == "RespondwithUDP"{
                            println!("RespondwithUDP");
                            handleNewUDPSocket(resp.sender.clone(), resp.data);         
                        } else if resp.header == "Connect"{
                            println!("Connect");
                            
                            if let Ok(device) = serde_json::from_str::<Device>(&resp.data) {
                                unsafe {
                                    DEVICENAMESMAP.insert(
                                        resp.sender.to_string(),
                                        device,
                                    ); 
                                    CONNECTEDTOPEERS = true;
                                };
                            }
                            unsafe{
                                let msg = Message {
                                    sender: PEER_ID.to_string(),
                                    header: "RespondConnect".to_string(),
                                    data: serde_json::to_string(&DEVICENAMESMAP.get(&PEER_ID.clone().to_string())).expect("can jsonify request"),
                                    receiver: vec![resp.sender]
                                };
                                if let Err(e) = self.response_sender.send(msg) {
                                    error!("error sending response via channel, {}", e);
                                }
                            }
                        } else if resp.header == "RespondConnect"{
                            if let Ok(device) = serde_json::from_str::<Device>(&resp.data) {
                                unsafe {
                                    DEVICENAMESMAP.insert(
                                        resp.sender.to_string(),
                                        device,
                                    ); 
                                    CONNECTEDTOPEERS = true;
                                };
                                
                            }
                        } else if resp.header == "UpdateSets"{
                            if let Ok(sets) = serde_json::from_str::<Vec<Set>>(&resp.data) {
                                unsafe {
                                    SETS = sets;
                                };
                            }
                        } else if resp.header == "StartSet"{
                            println!("got StartSet");
                            if let Ok(profile) = serde_json::from_str::<StartMessage>(&resp.data) {
                               let sender = self.response_sender.clone();
                                thread::spawn(move || {
                                    Set::startSet(profile.set,  sender)                              
                               });
                             }
                        }else if resp.header == "Unswap"{
                            unswap(resp.data);
                        }
                        
                        // resp.data.iter().for_each(|r| info!("{:?}", r));
                    }
                } 
            },
            _ => {
                unsafe {
                    if !PUBLISHEDUDP{
                        println!("publushubg");
                        publishUDPSocket( self.response_sender.clone());
                        PUBLISHEDUDP = true;
                    }
                }
            }
        }
    }
}

fn handleNewUDPSocket(sender: String, udpAddr: String ){
    println!("Logging UDP socket");
    unsafe{ 
        UDPMAP.insert(
            sender.to_string(),
            udpAddr.to_string(),
        ); 
        println!("{:?}",UDPMAP);
    };

    
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


#[tokio::main]
async fn main() {
    println!("socket {:?}",UDPSOCKET);
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

    swarm.floodsub.publish(TOPIC.clone(), " ".to_string().as_bytes());
    let startup_sender = swarm.response_sender.clone();
    
    tokio::spawn(async move {
        unsafe {
            DEVICENAMESMAP.insert(
                PEER_ID.to_string(),
                Device{
                    name:whoami::devicename().to_string(),
                    mac_addr:mac_address::get_mac_address().unwrap().expect("got mac addr").bytes(),
                    os:whoami::platform().to_string()
                },
            ); 
            loop  {
                if !CONNECTEDTOPEERS{
                    println!("trying to connect....");
                    handle_start_up(startup_sender.clone());
                    thread::sleep( time::Duration::from_millis(5000));//try to connect every 5 sec
                }else{
                    break
                }
            }
        }
    });
    println!("here");
    thread::spawn(move || {
        loop {
            // println!("Receiving MOUSE MOVE!");
            let mut buf = [0; 4];
            let (amt, src) = UDPSOCKET.recv_from(&mut buf).unwrap();
            let buff = &mut buf[..amt];
            
            if buff[3] == (Wrapping(buff[0]as i8) + Wrapping(buff[1] as i8) + Wrapping(buff[2] as i8)).0 as u8{
                if buff[0] == 1{
                    // move_rel((buff[1] as i8) as i32,(buff[2]as i8) as i32);  
                    send_mouse_input(MOUSEEVENTF_MOVE,0,(buff[1] as i8) as i32,(buff[2]as i8) as i32)
                }else{
                    // send_mouse_input(intToMouseFlag(buff[0]),0,0,0)
                    send_mouse_input(intToMouseFlag(buff[0]),0,(buff[1] as i8) as i32,(buff[2]as i8) as i32)
                }
            }else{
                println!("WOW THATS A SURPRISE!!!")
            }
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
                EventType::Response(mut resp) => {
                    if resp.receiver.len() == 0 {
                        println!("public");
                        println!("message: {:?}",resp);
                        resp.receiver = get_all_users(&mut swarm).await
                    }else{
                        // println!("private")
                    }
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
                }
                EventType::Input(line) => match line.as_str() {
                    "ls p" => handle_list_peers(&mut swarm).await,
                    // cmd if cmd.starts_with("swap") => handle_swap(cmd,swarm.response_sender.clone()),
                    cmd if cmd.starts_with("unswap") => handle_unswap(cmd,swarm.response_sender.clone()),
                    "ping" => handle_ping(&mut swarm).await,
                    "avarage" => get_avarage(),
                    cmd if cmd.starts_with("send test") => send_test(cmd, &mut swarm).await,
                    "devices" => handle_list_devices(),
                    cmd if cmd.starts_with("newset") => handle_newset(cmd,swarm.response_sender.clone()),
                    cmd if cmd.starts_with("newprofile") => handle_newprofile(cmd,swarm.response_sender.clone()),
                    "loadsets" => handle_loadsets(swarm.response_sender.clone()).await,
                    "savesets" => handle_savesets(swarm.response_sender.clone()).await,
                    cmd if cmd.starts_with("editprofile") => handle_editprofile(cmd,swarm.response_sender.clone()),
                    cmd if cmd.starts_with("startset") => handle_startset(cmd,swarm.response_sender.clone()),
                    cmd if cmd.starts_with("startprofile") => handle_startprofile(cmd,swarm.response_sender.clone()),
                    // cmd if cmd.starts_with("viewset") => handle_viewset(cmd),
                    // cmd if cmd.starts_with("viewprofile") => handle_viewprofile(cmd),
                    cmd if cmd.starts_with("editorder") => handle_editorder(cmd,swarm.response_sender.clone()),
                    cmd if cmd.starts_with("setexit") => handle_setexit(cmd,swarm.response_sender.clone()),
                    cmd if cmd.starts_with("setcycle") => handle_setcycle(cmd,swarm.response_sender.clone()),
                    cmd if cmd.starts_with("setname") => handle_changename(cmd,swarm.response_sender.clone()),
                    cmd if cmd.starts_with("viewsets") => handle_viewsets(),
                    _ => error!("unknown command"),
                },
            }
        }
    }
}

fn handle_list_devices(){
    unsafe {println!("Devices: {:?}",DEVICENAMESMAP);}
}

fn handle_start_up(sender:  mpsc::UnboundedSender<Message>) {
    unsafe {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "Connect".to_string(),
            data: serde_json::to_string(&DEVICENAMESMAP.get(&PEER_ID.clone().to_string())).expect("can jsonify request"),                   
            receiver: vec![]
        };
        // println!("msg {:?}",msg);
        // let json = serde_json::to_string(&msg).expect("can jsonify request");
        // swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
        sender.send(msg).expect("sent msg");
    }
}

 fn publishUDPSocket(sender:  mpsc::UnboundedSender<Message>){
     println!("Local Addr: {}", UDPSOCKET.local_addr().unwrap());
    let msg = Message {
        sender: PEER_ID.to_string(),
        header: "PublishUDP".to_string(),
        data: UDPSOCKET.local_addr().unwrap().to_string(),
        receiver: vec![]
    };
    sender.send(msg).expect("sent msg");
}
fn get_avarage(){
    unsafe{
        let mut sum1 = 0;
        for f in &KEYBOARDAVARAGE {
            sum1 += f;
        }
        println!("keyboard avarage {},",sum1/KEYBOARDAVARAGE.len() as i64);
        let mut sum2 = 0;
        for f in &MOUSEAVARAGE {
            sum2 += f;
        }
        println!("keyboard avarage {},",sum2/MOUSEAVARAGE.len() as i64);
    }
    
}

async fn handle_ping(swarm: &mut Swarm<RecipeBehaviour>) {
    let dt = chrono::prelude::Local::now();
    let milliseconds: i64= dt.timestamp_millis();
    let msg = Message {
        sender: PEER_ID.to_string(),
        header: "Ping".to_string(),
        data: milliseconds.to_string(),
        receiver: get_all_users(swarm).await
    };
    let json = serde_json::to_string(&msg).expect("can jsonify request");
    swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
    
}

fn init(sender:  mpsc::UnboundedSender<Message>){
    unsafe {set_block_keyboard(false);}
    thread::spawn( move || {
        receive_keyboard_event();
    });
    let keyboard_listener = get_keyboard_recv();
    let keyboard_sender = sender.clone();
    thread::spawn( move || {
        for key_event_struct in keyboard_listener.iter() {
            handle_keyboard_event(key_event_struct,keyboard_sender.clone());
        }
    });
    unsafe {set_block_mouse(false);}
    thread::spawn( move || {
        receive_mouse_event();
    });
    let mouse_listener = get_mouse_recv();    
    let mouse_sender = sender.clone();
    thread::spawn( move || {
        for mouse_event_struct in mouse_listener.iter() {
            handle_mouse_event(mouse_event_struct,mouse_sender.clone());
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

fn handle_terminate(sender:  mpsc::UnboundedSender<Message>){
    println!("unswappping......");
    println!("unswappping......");
    println!("unswappping......");
    handle_unswap("unswap mouse keyboard",sender.clone());
}

fn handle_cycle(sender:  mpsc::UnboundedSender<Message>){
    
    let set  = CURRENTSET.lock().expect("Could not lock mutex").clone();
    if set != ""{
        println!("cycling....");
        Set::cycleProfiles(CURRENTSET.lock().expect("Could not lock mutex").clone(), sender.clone());
    }
}

fn set_currentset(newset: String){
    println!("here??? in currentset");
    let mut state = CURRENTSET.lock().unwrap();
    *state = newset;
    println!("here!!! in currentset");
}

fn set_keyboard_block(state:bool){
    unsafe {set_block_keyboard(state);}
}

fn set_keyboard_recivers(recivers: Vec<String>){
    // KEYBOARD_RECIEVERS = Mutex::new(recivers);
    let mut state = KEYBOARD_RECIEVERS.lock().expect("Could not lock mutex");
    *state = recivers;
}

fn set_mouse_recivers(recivers: Vec<String>){
    let mut state = MOUSE_RECIEVERS.lock().expect("Could not lock mutex");
    *state = recivers;
}

fn set_mouse_block(state:bool){
    unsafe {set_block_mouse(state);}
}

fn handle_keyboard_event(key_event_struct: KBDLLHOOKSTRUCT, sender:  mpsc::UnboundedSender<Message>){
    let temp_receivers = KEYBOARD_RECIEVERS.lock().expect("Failed to unlock Mutex").clone();
    if temp_receivers.len() != 0{
        let dt = chrono::prelude::Local::now();
        let milliseconds: i64= dt.timestamp_millis();
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "KeyboardEvent".to_string(),
            data: serde_json::to_string(&KeyboardEvent{vkCode:key_event_struct.vkCode,scanCode:key_event_struct.scanCode,flags:key_event_struct.flags,time: milliseconds }).expect("can jsonify request"),
            receiver: temp_receivers,
        };
        if let Err(e) = sender.send(msg) {
            error!("error sending response via channel, {}", e);
        }
    }
    
}

fn handle_mouse_event(mut mouse_event_struct: MouseEvent, sender:  mpsc::UnboundedSender<Message>){
    let recivers = MOUSE_RECIEVERS.lock().expect("Failed to unlock Mutex").clone();
    if recivers.len() != 0 {
        let intType = mouseFlagToInt(mouse_event_struct.flags);
        if mouse_event_struct.flags == 0{
            let mut buff:[u8;4] = [0;4];
            buff[0] = 1 as u8;
            buff[1] = (mouse_event_struct.pt.0 as i8) as u8;
            buff[2] = (mouse_event_struct.pt.1 as i8) as u8;
            buff[3] =  (Wrapping(buff[0]as i8) + Wrapping(buff[1] as i8) + Wrapping(buff[2] as i8)).0 as u8;
            
            unsafe {
                for (key, value) in &*UDPMAP {
                    if recivers.iter().any(|i| i == key) {
                        UDPSOCKET.send_to(&buff, &value.clone()).unwrap();
                    }
                    
                }
            }    
        }else if intType != 0{
            let mut buff:[u8;4] = [0;4];
            buff[0] = intType;
            buff[1] = 0;
            buff[2] = 0;
            buff[3] =  intType;
            unsafe {
                for (key, value) in &*UDPMAP {
                    println!("Sending mouse btn");
                    if recivers.iter().any(|i| i == key) {
                        UDPSOCKET.send_to(&buff, &value.clone()).unwrap();
                    }
                }
            }  
        }else{
            let dt = chrono::prelude::Local::now();
            let milliseconds: i64= dt.timestamp_millis();
            mouse_event_struct.time = milliseconds;
            let msg = Message {
                sender: PEER_ID.to_string(),
                header: "MouseEvent".to_string(),
                data: serde_json::to_string(&mouse_event_struct).expect("can jsonify request"),
                receiver: recivers,
            };
            println!("Sending mouse sidebtn");
            if let Err(e) = sender.send(msg) {
                error!("error sending response via channel, {}", e);
            }
        }
    }
    
}
fn swap_keyboard(sender: mpsc::UnboundedSender<Message>, receivers: Vec<String>){
    println!("swapping keyboard");
    let keyboard_sender = sender.clone();
    let mut temp = receivers.clone();
    if receivers.iter().any(|i| (*i) == PEER_ID.to_string()) {
        unsafe {set_block_keyboard(false);}
    }else{
        unsafe {set_block_keyboard(true);}
    }   
    thread::spawn( move || {
        receive_keyboard_event();
    });
    let keyboard_listener = get_keyboard_recv();
    temp.retain(|x| (*x) != PEER_ID.to_string());
    if temp.len() == 0{
        println!("RETURNING!!!!!!!!!!");
        return 
    }
    println!("REMAINIGN TEMP, {:?}", temp);
    thread::spawn(move || {
        for key_event_struct in keyboard_listener.iter() {
            // println!("key_event_struct: code123: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",key_event_struct.vkCode, key_event_struct.scanCode,key_event_struct.flags,key_event_struct.time,key_event_struct.dwExtraInfo);
            //Comment
            let dt = chrono::prelude::Local::now();
            let milliseconds: i64= dt.timestamp_millis();
            let temp_receivers = temp.clone();
            let msg = Message {
                sender: PEER_ID.to_string(),
                header: "KeyboardEvent".to_string(),
                data: serde_json::to_string(&KeyboardEvent{vkCode:key_event_struct.vkCode,scanCode:key_event_struct.scanCode,flags:key_event_struct.flags,time: milliseconds }).expect("can jsonify request"),
                receiver: temp_receivers,
            };
            if let Err(e) = keyboard_sender.send(msg) {
                error!("error sending response via channel, {}", e);
            }
            // let json = serde_json::to_string(&msg).expect("can jsonify request");
            // swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
        }
    });
}

fn mouseFlagToInt(flag: DWORD) -> u8{
    return match flag {
        MOUSEEVENTF_LEFTDOWN => 2,
        MOUSEEVENTF_RIGHTDOWN => 3,
        MOUSEEVENTF_MIDDLEDOWN => 4,
        MOUSEEVENTF_LEFTUP => 5,
        MOUSEEVENTF_RIGHTUP => 6,
        MOUSEEVENTF_MIDDLEUP  => 7,
        _ => 0,
    }
}

fn intToMouseFlag(flag: u8) -> DWORD{
    return match flag {
        2 => MOUSEEVENTF_LEFTDOWN,
        3 => MOUSEEVENTF_RIGHTDOWN,
        4 => MOUSEEVENTF_RIGHTDOWN,
        5 => MOUSEEVENTF_LEFTUP,
        6 => MOUSEEVENTF_RIGHTUP,
        7 => MOUSEEVENTF_MIDDLEUP,
        _ => 0,
    }
}

fn swap_mouse(sender: mpsc::UnboundedSender<Message>, receivers: Vec<String>){
    println!("Swapping mouse!");
    let mouse_sender = sender.clone();
    let mut temp = receivers.clone();
    if receivers.iter().any(|i| (*i) == PEER_ID.to_string()) {
        unsafe {set_block_mouse(false);}
    }else{
        unsafe {set_block_mouse(true);}
    }
    thread::spawn( move || {
        
        receive_mouse_event();
    });
    let mouse_listener = get_mouse_recv();    
    temp.retain(|x| (*x) != PEER_ID.to_string());
    if temp.len() == 0{
        return 
    }
    thread::spawn( move || {
        for mut mouse_event_struct in mouse_listener.iter() {
            let intType = mouseFlagToInt(mouse_event_struct.flags);
            if mouse_event_struct.flags == 0{
                let mut buff:[u8;4] = [0;4];
                buff[0] = 1 as u8;
                buff[1] = (mouse_event_struct.pt.0 as i8) as u8;
                buff[2] = (mouse_event_struct.pt.1 as i8) as u8;
                buff[3] =  (Wrapping(buff[0]as i8) + Wrapping(buff[1] as i8) + Wrapping(buff[2] as i8)).0 as u8;
                
                unsafe {
                    for (key, value) in &*UDPMAP {
                        if temp.iter().any(|i| i == key) {
                            UDPSOCKET.send_to(&buff, &value.clone()).unwrap();
                        }
                        
                    }
                }    
            }else if intType != 0{
                let mut buff:[u8;4] = [0;4];
                buff[0] = intType;
                buff[1] = 0;
                buff[2] = 0;
                buff[3] =  intType;
                unsafe {
                    for (key, value) in &*UDPMAP {
                        println!("Sending mouse btn");
                        if temp.iter().any(|i| i == key) {
                            UDPSOCKET.send_to(&buff, &value.clone()).unwrap();
                        }
                    }
                }  
            }else{
                let temp_receivers = temp.clone();
                let dt = chrono::prelude::Local::now();
                let milliseconds: i64= dt.timestamp_millis();
                mouse_event_struct.time = milliseconds;
                let msg = Message {
                    sender: PEER_ID.to_string(),
                    header: "MouseEvent".to_string(),
                    data: serde_json::to_string(&mouse_event_struct).expect("can jsonify request"),
                    receiver: temp_receivers,
                };
                println!("Sending mouse sidebtn");
                if let Err(e) = mouse_sender.send(msg) {
                    error!("error sending response via channel, {}", e);
                }
            }
            
            // let json = serde_json::to_string(&msg).expect("can jsonify request");
            // swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
        }
    });
}

fn unswap(rest:String){
    let elements: Vec<&str> = rest.split(" ").collect();
        for f in &elements {
           match f {
            &"mouse" => {
                set_mouse_block(false);
                set_mouse_recivers(vec![]);
            },
            &"keyboard" => {
                set_keyboard_block(false);
                set_keyboard_recivers(vec![]);
            },
            invalid =>{
                println!("Invalid item to swap: {}", invalid)
            }
           }
        }
}

fn handle_unswap(cmd: &str, sender:  mpsc::UnboundedSender<Message>){
    if let Some(rest) = cmd.strip_prefix("unswap") {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "Unswap".to_string(),
            data: rest.to_owned(),
            receiver: vec![]
        };
        if let Err(e) = sender.send(msg) {
            error!("error sending response via channel, {}", e);
        }else{
            unswap(rest.to_owned());
        }
    }
}

// fn handle_swap(cmd: &str ,sender: mpsc::UnboundedSender<Message>) {
//     // thread::spawn(move || {
//     //     receive_keyboard_event(); 
//     // });
//     // receive_mouse_event();
    
//     // let msg = Message {
//     //     header: "KeyboardEvent".to_string(),
//     //     data: "lolol".to_string(),
//     //     receiver: vec![],
//     // };
//     // if let Err(e) = sender.clone().send(msg) {
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
    let mut array : Vec<String> = Vec::new();
    for peer in nodes {
        if !array.contains(&peer.to_string()){
            array.push(peer.to_string());
        }
    }
    array
}

async fn send_test(cmd: &str ,swarm: &mut Swarm<RecipeBehaviour>) {
    let sender = swarm.response_sender.clone();
    if let Some(rest) = cmd.strip_prefix("send test") {
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "Test".to_string(),
            data: rest.to_string(),
            receiver: get_all_users(swarm).await
        };
        let json = serde_json::to_string(&msg).expect("can jsonify request");
        swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
    }
}

fn handle_newset(cmd: &str,sender:  mpsc::UnboundedSender<Message>){
    if let Some(rest) = cmd.strip_prefix("newset") {
        if Set::new(rest.trim().to_owned(),sender.clone()) {
            println!("success!");
            return
        }
    }
    println!("failure to create new set!");
}
fn handle_newprofile(cmd: &str,sender:  mpsc::UnboundedSender<Message>){
    if let Some(rest) = cmd.strip_prefix("newprofile"){
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        if Set::newProfile(elements[0].trim().to_owned(),elements[1].trim().to_owned(),sender.clone()) {
            println!("success!");
            return
        }
    }
    println!("failure to create new set!");
}

fn handle_editprofile(cmd: &str,sender:  mpsc::UnboundedSender<Message>){
    if let Some(rest) = cmd.strip_prefix("editprofile"){
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        let mut receivers  = vec![];
        let mut sender_id = elements[3].trim().to_owned();
        match searchDeviceName(elements[3].trim().to_owned()).as_str()  { 
            "" => (),
            a => sender_id = a.to_owned()
        };
        for i in 4..elements.len(){
            match searchDeviceName(elements[i].trim().to_owned()).as_str()  { 
                "" => receivers.push(elements[i].trim().to_owned()),
                a => receivers.push(a.to_owned()),
            };
        }
        Set::editProfile(elements[0].trim().to_owned(), elements[1].trim().to_owned(),elements[2].trim().to_owned(), sender_id, receivers, sender.clone())
    }
}

fn handle_changename(cmd: &str,sender: mpsc::UnboundedSender<Message> ){
    if let Some(rest) = cmd.strip_prefix("setname"){
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "RespondConnect".to_string(),
            data: serde_json::to_string(
                &Device{
                    name: rest.trim().to_owned(),
                    mac_addr:mac_address::get_mac_address().unwrap().expect("got mac addr").bytes(),
                    os:whoami::platform().to_string()
                }).expect("can jsonify request"),
            receiver: vec![]
        };
        if let Err(e) = sender.send(msg) {
            error!("error sending response via channel, {}", e);
        }else {
            unsafe {
                DEVICENAMESMAP.insert(
                    PEER_ID.clone().to_string(),
                    Device{
                        name: rest.trim().to_owned(),
                        mac_addr:mac_address::get_mac_address().unwrap().expect("got mac addr").bytes(),
                        os:whoami::platform().to_string()
                    },
                ); 
            };
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
struct StartMessage{
    set: String,
    profile: String,
}
fn handle_startset(cmd: &str, sender:  mpsc::UnboundedSender<Message>){
    updateSet(sender.clone());
    if let Some(rest) = cmd.strip_prefix("startset"){
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "StartSet".to_string(),
            data: serde_json::to_string(&StartMessage{set:elements[0].to_owned(), profile: Set::getfirstprofile(elements[0].to_owned())}).expect("can jsonify request"),
            receiver: vec![]
        };
        if let Err(e) = sender.clone().send(msg) {
            error!("error sending response via channel, {}", e);
        }else{
            println!("starting local????");
            let set_id = elements[0].to_owned();
            thread::spawn(move || {
                Set::startSet(set_id, sender.clone());
            });
        }
    }  
}

fn handle_startprofile(cmd: &str, sender:  mpsc::UnboundedSender<Message>){
    updateSet(sender.clone());
    if let Some(rest) = cmd.strip_prefix("startprofile"){
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        let msg = Message {
            sender: PEER_ID.to_string(),
            header: "StartSet".to_string(),
            data: serde_json::to_string(&StartMessage{set:elements[0].to_owned(), profile: elements[1].to_owned()}).expect("can jsonify request"),
            receiver: vec![]
        };
        if let Err(e) = sender.send(msg) {
            error!("error sending response via channel, {}", e);
        }else{//TODO i think this is blocking :/
            Set::startProfile(elements[0].to_owned(), elements[1].to_owned(), sender.clone())
        }
    }  
}


async fn handle_loadsets(sender:  mpsc::UnboundedSender<Message>){
    let loadedset = Set::loadFromDefaultFile().await;
    println!("loadedset: {:?}", loadedset);
    if loadedset.len() != 0{
        unsafe{SETS = loadedset}
        updateSet(sender.clone());
    }
    
    
}

async fn handle_savesets(sender:  mpsc::UnboundedSender<Message>){
    Set::saveToDefaultFile().await;
    println!("saved!");
}

fn handle_viewsets(){
    unsafe{println!("SETS {:?}", SETS);}
}
fn handle_setexit(cmd: &str, sender:  mpsc::UnboundedSender<Message>){
    if let Some(rest) = cmd.strip_prefix("setexit"){
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        Set::setexit(elements[0].to_owned(), elements[1..].to_vec().iter().map(|x| (*x).to_owned().parse::<u32>().unwrap()).collect(), sender.clone());
    }
}
fn handle_setcycle(cmd: &str, sender:  mpsc::UnboundedSender<Message>){
    if let Some(rest) = cmd.strip_prefix("setcycle"){
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        Set::setcycle(elements[0].to_owned(), elements[1..].to_vec().iter().map(|x| (*x).to_owned().parse::<u32>().unwrap()).collect(), sender.clone());
    }
}
fn handle_editorder(cmd: &str, sender:  mpsc::UnboundedSender<Message>){
    //TODO
    if let Some(rest) = cmd.strip_prefix("editorder"){
        let elements: Vec<&str> = rest.trim().split(" ").collect();
        Set::editorder(elements[0].to_owned(), elements[1..].to_vec().iter().map(|x| (*x).to_owned()).collect(), sender.clone());
    }
}