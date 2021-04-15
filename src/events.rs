

pub mod events{
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct LinuxEvent{
        pub type_:u16,
        pub code: u16,
        pub value: i32
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct KeyboardEvent {
        pub vkCode: u32,
        pub scanCode: u32,
        pub flags: u32,
        pub time: i64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MouseEvent {
        pub pt: (i32, i32),
        pub mouseData: u32,
        pub flags: u32,
        pub time: i64,
    }
    


    //linux codes  
    //mouse move - type 2 
        // x = 0, value is pixels
        // y = 1 "               "
    //mouse btn from  right to left
        //273
        //274
        //272
        //276
        //275


    //from 4bit input to Linux event
    pub fn from_windows_mouse_event(buff: &mut [u8]) -> Vec<LinuxEvent>{
        let mut linux_events = vec![];
        if buff[0] == 1 {
            // move_rel((buff[1] as i8) as i32,(buff[2]as i8) as i32);
            linux_events.push(LinuxEvent{type_: 2, code: 0 , value: (buff[1] as i8) as i32});
            linux_events.push(LinuxEvent{type_: 2, code: 1 , value: (buff[2] as i8) as i32});
            
        } else {
            linux_events.push(LinuxEvent{type_: 4, code: LinuxintToMouseFlag(buff[0]).0 , value: LinuxintToMouseFlag(buff[0]).1});
            // send_mouse_input(intToMouseFlag(buff[0]),0,0,0)
            // send_mouse_input(
            //     intToMouseFlag(buff[0]),
            //     0,
            //     (buff[1] as i8) as i32,
            //     (buff[2] as i8) as i32,
            // )
        }

        // LinuxEvent{type_: 1, code: code as u16, value: ((1 + (event.flags >> 7)) % 2) as i32}
        linux_events
    }

    fn LinuxintToMouseFlag(flag: u8) -> (u16,i32) {
        return match flag {
            2 => (272,1),
            3 => (273,1),
            4 => (274,1),
            5 => (272,0),
            6 => (273,0),
            7 => (274,1),
            _ => (0,0),
        };
    }

    fn LinuxFromMouseFlag(flags: (u16,i32)) ->u8 {
        return match flags {
            (272,1) => 2,
            (273,1) => 3,
            (274,1) => 4,
            (272,0) => 5,
            (273,0) => 6,
            (274,0) => 7,
            _ => 0,
        };
    }

    use std::num::Wrapping;
    // to 4 but input 
    pub fn from_linux_mouse_event(event: &LinuxEvent, buffer: (i32,i32)) -> [u8; 4]{
        let mut buff = [0; 4];
        if event.type_ == 2{
            buff[0] = 1 as u8;
            if event.code == 0{
                buff[1] = (event.value as i8) as u8 + buffer.0 as u8;
                buff[2] = buffer.1 as u8;
                buff[3] = (Wrapping(buff[0] as i8) + Wrapping(buff[1] as i8) + Wrapping(buff[2] as i8)).0 as u8;
                return buff
            }
            if event.code == 1{
                buff[1] = buffer.0 as u8;
                buff[2] = (event.value as i8) as u8  + buffer.1 as u8;
                buff[3] = (Wrapping(buff[0] as i8) + Wrapping(buff[1] as i8) + Wrapping(buff[2] as i8)).0 as u8;
                return buff
            }
            
        }else if event.type_ == 1{
            buff[0] = LinuxFromMouseFlag((event.code,event.value));
            buff[1] = 0;
            buff[2] = 0;
            buff[3] = (Wrapping(buff[0] as i8) + Wrapping(buff[1] as i8) + Wrapping(buff[2] as i8)).0 as u8;
            return buff
        }
        return buff
    }



    pub fn from_linux_keyboard_event(event: LinuxEvent) -> KeyboardEvent{
        let code = linux_to_windows_keyboard_event(event.code.into());
        let mut flag = 0;
        if event.value == 0{
            flag = 128
        }
        KeyboardEvent{vkCode: 0,scanCode:code, flags: flag,  time:0}
    }


    pub fn from_windows_keyboard_event(event: KeyboardEvent) -> LinuxEvent{
        let code = windows_to_linux_keyboard_event(event.scanCode);

        LinuxEvent{type_: 1, code: code as u16, value: ((1 + (event.flags >> 7)) % 2) as i32}
    }

    fn windows_to_linux_keyboard_event(code: u32) -> u32{
        if code <= 0 {
            return 0
        }
        if code <= 88 {
            return code
        }
        if code <= 95 {
            return 0//OEM stuff 
        }
        match code{ 
            98 => return 418,
            99 => return 138,
            _ => (),
        }
        if code <= 110 && code >= 100{
            return code + 83
        }
        match code{
            118 => return 194,
            112 => return 90,
            57360 => return 165,
            57369 => return 163,
            57372 => return 96,
            57373 => return 97,
            57376 => return 113,
            57378 => return 164,
            57380 => return 166,
            57390 => return 114,
            57392 => return 115,
            57394 => return 172,
            57397 => return 98,
            57399 => return 99,
            57400 => return 100,
            57414 => return 223,
            57415 => return 172,
            57416 => return 103,
            57417 => return 104,
            57419 => return 105,
            57421 => return 106,
            57423 => return 107,
            57424 => return 108,
            57425 => return 109,
            57426 => return 110,
            57427 => return 111,
            57435 => return 125,
            57436 => return 126,
            57438 => return 116,
            57439 => return 142,
            57443 => return 143,
            57445 => return 217,
            57446 => return 364,
            57447 => return 173,
            57448 => return 128,
            57449 => return 159,//check
            57450 => return 158,//check
            57451 => return 0,//check
            57452 => return 215,
            57453 => return 226,
            _ => (),
        }
        return 0//randoms
    }

    fn linux_to_windows_keyboard_event(code: u32) -> u32{
        if code <= 0 {
            return 0
        }
        if code <= 88 {
            return code
        }
        if code >= 183 && code <= 193{
            return code + 83
        }
        match code{
            418 => return 98,
            138 => return 99,
            194 => return 118,
            90 => return 112 ,
            165 => return 57360,
            163 => return 57369,
            96 => return 57372,
            97 => return 57373,
            113 => return 57376,
            164 => return 57378,
            166 => return 57380,
            114 => return 57390,
            115 => return 57392,
            172 => return 57394,
            98 => return 57397,
            99 => return 57399,
            100 => return 57400,
            223 => return 57414,
            172 => return 57415,
            103 => return 57416,
            104 => return 57417,
            105 => return 57419,
            106 => return 57421,
            107 => return 57423,
            108 => return 57424,
            109 => return 57425,
            110 => return 57426,
            111 => return 57427,
            125 => return 57435,
            126 => return 57436,
            116 => return 57438,
            142 => return 57439,
            143 => return 57443,
            217 => return 57445,
            364 => return 57446,
            173 => return 57447,
            128 => return 57448,
            159 => return 57449,
            158 => return 57450,
            215 => return 57452,
            226 => return 57453,
            _ => (),
        }
        return 0//randoms
    }
}

