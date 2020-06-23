use nom::{
    bytes::streaming::take_till,
    number::streaming::{le_f32, le_u8},
};
use nom_derive::*;
use std::{
    ffi::{CStr, CString},
    time::Duration,
};

fn bytes_to_cstring(slice: &[u8]) -> CString {
    unsafe { CStr::from_bytes_with_nul_unchecked(slice).to_owned() }
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct A2SPlayer {
    pub index: u8,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub name: CString,
    pub score: i32,
    #[nom(Parse = "le_f32", Map = "Duration::from_secs_f32")]
    pub duration: Duration,
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct ModData {
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub link: CString,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub download_link: CString,
    #[nom(AlignBefore(1))]
    pub version: i32,
    pub size: i32,
    #[nom(Parse = "le_u8", Map = "|x: u8| x != 0")]
    pub mp_only: bool,
    #[nom(Parse = "le_u8", Map = "|x: u8| x != 0")]
    pub custom_dll: bool,
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct A2SInfoOld {
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub address: CString,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub name: CString,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub map: CString,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub folder: CString,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub game: CString,
    pub players: u8,
    pub max_players: u8,
    pub protocol: u8,
    pub server_type: u8,
    pub enviroment: u8,
    #[nom(Parse = "le_u8", Map = "|x: u8| x != 0")]
    pub is_private: bool,
    mod_data_exists: u8,
    #[nom(Cond = "mod_data_exists == 1")]
    pub mod_data: Option<ModData>,
    #[nom(Parse = "le_u8", Map = "|x: u8| x != 0")]
    pub vac_secured: bool,
    pub bots_num: u8,
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct ExtraData {
    pub edf: u8,
    #[nom(Cond = "edf & 0x80 != 0")]
    pub port: Option<i16>,
    #[nom(Cond = "edf & 0x10 != 0")]
    pub server_steamid: Option<u64>,
    #[nom(Cond = "edf & 0x40 != 0")]
    pub port_source_tv: Option<i16>,
    #[nom(
        Cond = "edf & 0x40 != 0",
        Parse = "take_till(|b| b == 0)",
        Map = "bytes_to_cstring"
    )]
    pub name_source_tv: Option<CString>,
    #[nom(
        Cond = "edf & 0x20 != 0",
        Parse = "take_till(|b| b == 0)",
        Map = "bytes_to_cstring"
    )]
    pub keywords: Option<CString>,
    #[nom(Cond = "edf & 0x01 != 0")]
    pub gameid: Option<u64>,
}
#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct A2SInfoNew {
    pub protocol: u8,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub name: CString,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub map: CString,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub folder: CString,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub game: CString,
    pub steamid: i16,
    pub players: u8,
    pub max_players: u8,
    pub bots: u8,
    pub server_type: u8,
    pub enviroment: u8,
    #[nom(Parse = "le_u8", Map = "|x: u8| x != 0")]
    pub is_visible: bool,
    #[nom(Parse = "le_u8", Map = "|x: u8| x != 0")]
    pub vac_secured: bool,
    #[nom(Parse = "take_till(|b| b == 0)", Map = "bytes_to_cstring")]
    pub version: CString,
    pub extra_data: ExtraData,
}
