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
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::{fs, io::AsyncBufReadExt, sync::mpsc};


type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("Capstone"));

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use crate::windows::*;

#[derive(Debug, Serialize, Deserialize)]
struct Message {
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
                if let Ok(resp) = serde_json::from_slice::<Message>(&msg.data) {
                    if resp.receiver.contains(&PEER_ID.to_string()){
                        if resp.header == "Test".to_string() {
                            println!("perfect. Data: {:?} ",resp.data);
                        }
                        else if resp.header == "KeyboardEvent".to_string() {
                            if let Ok(keyboard_event_struct) = serde_json::from_str::<KeyboardEvent>(&resp.data) {     
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
                                }else if keyboard_event_struct.flags >> 5 % 2 ==  1{//make sure
                                    send_keybd_input(0,keyboard_event_struct.vkCode,KEYEVENTF_UNICODE );                      
                                }else if keyboard_event_struct.flags >> 7 % 2 ==  1{//make sure
                                    send_keybd_input(0,keyboard_event_struct.vkCode,KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);                      
                                }else{
                                    println!("SOMETGHING DIFFERENT  {:?}",keyboard_event_struct.flags);
                                    send_keybd_input(0,keyboard_event_struct.vkCode,KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);                      
                                }
                                //unhandled events : 160,161,33 (33 and 161 r together idfk 160)
                            }
                        }else if resp.header == "MouseEvent".to_string() {
                            
                            if let Ok(mouse_event_struct) = serde_json::from_str::<MouseEvent>(&resp.data) {  
                                 
                                if mouse_event_struct.flags != 0 {
                                    match mouse_event_struct.flags{
                                        MOUSEEVENTF_XDOWN|MOUSEEVENTF_XUP => send_mouse_input(mouse_event_struct.flags,mouse_event_struct.mouseData>>16,0,0),
                                        MOUSEEVENTF_WHEEL|MOUSEEVENTF_HWHEEL =>send_mouse_input(mouse_event_struct.flags,if 7864320 == mouse_event_struct.mouseData {120} else {(120*-1) as u32},0,0),
                                        _=> send_mouse_input(mouse_event_struct.flags,0,0,0)
                                    }
                                    
                                }else{
                                    move_rel(mouse_event_struct.pt.0,mouse_event_struct.pt.1)
                                }
                                //unhandled events : 160,161,33 (33 and 161 r together idfk 160)
                            }
                        }
                        
                        // resp.data.iter().for_each(|r| info!("{:?}", r));
                    }
                } 
            }
            _ => (),
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


#[tokio::main]
async fn main() {
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

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
    .expect("swarm can be started");
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
            println!("event {:?}", event);
            match event {
                EventType::Response(mut resp) => {
                    println!("Here!");
                    if resp.header == "KeyboardEvent".to_string() || resp.header == "MouseEvent".to_string() {
                        println!("Here2!");
                        resp.receiver = get_all_users(&mut swarm).await
                    }
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
                }
                EventType::Input(line) => match line.as_str() {
                    "ls p" => handle_list_peers(&mut swarm).await,
                    "swap" => handle_swap(swarm.response_sender.clone()),
                    cmd if cmd.starts_with("send test") => send_test(cmd, &mut swarm).await,
                    _ => error!("unknown command"),
                },
            }
        }
    }
}
fn handle_swap(sender: mpsc::UnboundedSender<Message>) {
    // thread::spawn(move || {
    //     receive_keyboard_event(); 
    // });
    // receive_mouse_event();
    
    // let msg = Message {
    //     header: "KeyboardEvent".to_string(),
    //     data: "lolol".to_string(),
    //     receiver: vec![],
    // };
    // if let Err(e) = sender.clone().send(msg) {
    //     error!("error sending response via channel, {}", e);
    // }
    let keyboard_sender = sender.clone();
    let mouse_sender = sender.clone();
    
    tokio::spawn(async move {
        let keyboard_listener = get_keyboard_recv();
        thread::spawn( move || {
            unsafe {set_block_keyboard(true);}
            receive_keyboard_event();
        });
        for key_event_struct in keyboard_listener.iter() {
            println!("key_event_struct: code123: {:?},  scanCode: {:?}, flags: {:?}, time: {:?}, extra: {:?},",key_event_struct.vkCode, key_event_struct.scanCode,key_event_struct.flags,key_event_struct.time,key_event_struct.dwExtraInfo);
            let msg = Message {
                header: "KeyboardEvent".to_string(),
                data: serde_json::to_string(&KeyboardEvent{vkCode:key_event_struct.vkCode,scanCode:key_event_struct.scanCode,flags:key_event_struct.flags, }).expect("can jsonify request"),
                receiver: vec![],
            };
            if let Err(e) = keyboard_sender.send(msg) {
                error!("error sending response via channel, {}", e);
            }
            // let json = serde_json::to_string(&msg).expect("can jsonify request");
            // swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
        }
    });
    tokio::spawn(async move {
        let mouse_listener = get_mouse_recv();
        thread::spawn( move || {
            unsafe {set_block_mouse(true);}
            receive_mouse_event();
        });
        for mouse_event_struct in mouse_listener.iter() {
            println!("GOT MESSAGE!");
            let msg = Message {
                header: "MouseEvent".to_string(),
                data: serde_json::to_string(&mouse_event_struct).expect("can jsonify request"),
                receiver: vec![],
            };
            if let Err(e) = mouse_sender.send(msg) {
                error!("error sending response via channel, {}", e);
            }
            // let json = serde_json::to_string(&msg).expect("can jsonify request");
            // swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
        }
    });
    

}
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
    let msg = Message {
        header: "KeyboardEvent".to_string(),
        data: "lolol".to_string(),
        receiver: vec![],
    };
    if let Err(e) = sender.clone().send(msg) {
        error!("error sending response via channel, {}", e);
    }
    if let Some(rest) = cmd.strip_prefix("send test") {
        let msg = Message {
            header: "Test".to_string(),
            data: rest.to_string(),
            receiver: get_all_users(swarm).await
        };
        let json = serde_json::to_string(&msg).expect("can jsonify request");
        swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
    }
}
