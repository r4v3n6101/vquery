use nom::number::streaming::le_f32;
use nom_derive::*;
use std::{ffi::CString, time::Duration};

fn take_cstring(i: &[u8]) -> nom::IResult<&[u8], CString> {
    let (i, cstr) = nom::bytes::streaming::take_till(|b| b == 0)(i)?;
    let (i, _) = nom::bytes::streaming::take(1usize)(i)?;
    // Safety: safe because we already know that cstr doesn't contain nul-byte
    let cstring = unsafe { CString::from_vec_unchecked(cstr.to_vec()) };
    Ok((i, cstring))
}

fn le_bool(i: &[u8]) -> nom::IResult<&[u8], bool> {
    nom::number::streaming::le_u8(i).map(|(i, b)| (i, b != 0))
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct ModData {
    #[nom(Parse = "take_cstring")]
    pub link: CString,
    #[nom(Parse = "take_cstring")]
    pub download_link: CString,
    #[nom(AlignBefore(1))]
    pub version: i32,
    pub size: i32,
    #[nom(Parse = "le_bool")]
    pub mp_only: bool,
    #[nom(Parse = "le_bool")]
    pub custom_dll: bool,
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct InfoOld {
    #[nom(Parse = "take_cstring")]
    pub address: CString,
    #[nom(Parse = "take_cstring")]
    pub name: CString,
    #[nom(Parse = "take_cstring")]
    pub map: CString,
    #[nom(Parse = "take_cstring")]
    pub folder: CString,
    #[nom(Parse = "take_cstring")]
    pub game: CString,
    pub players: u8,
    pub max_players: u8,
    pub protocol: u8,
    pub server_type: u8,
    pub enviroment: u8,
    #[nom(Parse = "le_bool")]
    pub is_private: bool,
    #[nom(
        PreExec = "let (i, mod_data_exists) = nom::number::streaming::le_u8(i)?;",
        Cond = "mod_data_exists == 1"
    )]
    pub mod_data: Option<ModData>,
    #[nom(Parse = "le_bool")]
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
    #[nom(Cond = "edf & 0x40 != 0", Parse = "take_cstring")]
    pub name_source_tv: Option<CString>,
    #[nom(Cond = "edf & 0x20 != 0", Parse = "take_cstring")]
    pub keywords: Option<CString>,
    #[nom(Cond = "edf & 0x01 != 0")]
    pub gameid: Option<u64>,
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct InfoNew {
    pub protocol: u8,
    #[nom(Parse = "take_cstring")]
    pub name: CString,
    #[nom(Parse = "take_cstring")]
    pub map: CString,
    #[nom(Parse = "take_cstring")]
    pub folder: CString,
    #[nom(Parse = "take_cstring")]
    pub game: CString,
    pub steamid: i16,
    pub players: u8,
    pub max_players: u8,
    pub bots: u8,
    pub server_type: u8,
    pub enviroment: u8,
    #[nom(Parse = "le_bool")]
    pub is_visible: bool,
    #[nom(Parse = "le_bool")]
    pub vac_secured: bool,
    #[nom(Parse = "take_cstring")]
    pub version: CString,
    pub extra_data: ExtraData,
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct Player {
    pub index: u8,
    #[nom(Parse = "take_cstring")]
    pub name: CString,
    pub score: i32,
    #[nom(Parse = "le_f32", Map = "Duration::from_secs_f32")]
    pub duration: Duration,
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct PlayersList {
    pub players_num: u8,
    pub players: Vec<Player>,
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct Rule {
    #[nom(Parse = "take_cstring")]
    pub key: CString,
    #[nom(Parse = "take_cstring")]
    pub value: CString,
}

#[derive(Debug, Nom)]
#[nom(LittleEndian)]
pub struct RulesList {
    pub rules_num: u16,
    pub rules: Vec<Rule>,
}
