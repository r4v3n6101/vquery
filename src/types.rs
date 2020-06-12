use byteorder::ByteOrder;
use byteorder_parser::ReadByteOrder;
use std::{
    ffi::CString,
    io::{Read, Result as IOResult},
};

#[derive(Debug, ReadByteOrder)]
pub struct A2SPlayer {
    pub index: u8,
    pub name: CString,
    pub score: i32,
    pub duration: f32,
}

#[derive(Debug, ReadByteOrder)]
pub struct ModData {
    pub link: CString,
    pub download_link: CString,
    _nul: u8,
    pub version: i32,
    pub size: i32,
    pub mp_only: bool,
    pub custom_dll: bool,
}

#[derive(Debug, ReadByteOrder)]
pub struct A2SInfoOld {
    pub address: CString,
    pub name: CString,
    pub map: CString,
    pub folder: CString,
    pub game: CString,
    pub players: u8,
    pub max_players: u8,
    pub protocol: u8,
    pub server_type: u8,
    pub enviroment: u8,
    pub is_private: bool,
    pub mod_data: Option<ModData>,
    pub vac_secured: bool,
    pub bots_num: u8,
}

#[derive(Debug)]
pub struct ExtraData {
    pub port: Option<i16>,
    pub server_steamid: Option<u64>,
    pub port_source_tv: Option<i16>,
    pub name_source_tv: Option<CString>,
    pub keywords: Option<CString>,
    pub gameid: Option<u64>,
}

impl ReadByteOrder for ExtraData {
    fn read_with_byteorder<O: ByteOrder, R: Read>(reader: &mut R) -> IOResult<Self> {
        let edf = u8::read_with_byteorder::<O, R>(reader)?;
        Ok(ExtraData {
            port: if edf & 0x80 != 0 {
                Some(i16::read_with_byteorder::<O, R>(reader)?)
            } else {
                None
            },
            server_steamid: if edf & 0x10 != 0 {
                Some(u64::read_with_byteorder::<O, R>(reader)?)
            } else {
                None
            },
            port_source_tv: if edf & 0x40 != 0 {
                Some(i16::read_with_byteorder::<O, R>(reader)?)
            } else {
                None
            },
            name_source_tv: if edf & 0x40 != 0 {
                Some(CString::read_with_byteorder::<O, R>(reader)?)
            } else {
                None
            },
            keywords: if edf & 0x20 != 0 {
                Some(CString::read_with_byteorder::<O, R>(reader)?)
            } else {
                None
            },
            gameid: if edf & 0x01 != 0 {
                Some(u64::read_with_byteorder::<O, R>(reader)?)
            } else {
                None
            },
        })
    }
}

#[derive(Debug, ReadByteOrder)]
pub struct A2SInfoNew {
    pub protocol: u8,
    pub name: CString,
    pub map: CString,
    pub folder: CString,
    pub game: CString,
    pub steamid: i16,
    pub players: u8,
    pub max_players: u8,
    pub bots: u8,
    pub server_type: u8,
    pub enviroment: u8,
    pub is_visible: bool,
    pub vac_secured: bool,
    pub version: CString,
    pub extra_data: ExtraData,
}
