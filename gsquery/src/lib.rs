extern crate byteorder;
extern crate either;
extern crate itertools;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use either::Either;
use itertools::Itertools;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    io::{BufRead, Cursor, Error as IOError, ErrorKind, Read, Result as IOResult},
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

const PACKET_SIZE: usize = 1400;

struct Packet {
    unique_id: i32,
    id: usize,
    packets_num: usize,
    data: Vec<u8>,
}

impl Packet {
    fn parse(packet: Vec<u8>) -> IOResult<Option<Packet>> {
        let mut cursor = Cursor::new(packet);
        let header = cursor.read_i32::<LittleEndian>()?;
        match header {
            -1 => {
                let mut data = Vec::new();
                cursor.read_to_end(&mut data)?;
                Ok(Some(Packet {
                    unique_id: 0,
                    id: 0,
                    packets_num: 1,
                    data,
                }))
            }
            -2 => {
                let id = cursor.read_i32::<LittleEndian>()?;
                let num = cursor.read_u8()?;
                let mut data = Vec::new();
                cursor.read_to_end(&mut data)?;
                Ok(Some(Packet {
                    unique_id: id,
                    id: (num & 0xF0 >> 4) as usize,
                    packets_num: (num & 0xF0) as usize,
                    data,
                }))
            }
            _ => Ok(None),
        }
    }
}

trait ReadCString: BufRead {
    fn read_cstring(&mut self) -> IOResult<CString> {
        let mut out: Vec<u8> = Vec::new();
        self.read_until(b'\0', &mut out)?;
        match CStr::from_bytes_with_nul(&out) {
            Ok(cstr) => Ok(CString::from(cstr)),
            Err(nul_err) => Err(IOError::new(ErrorKind::InvalidData, nul_err)),
        }
    }
}

impl<T: AsRef<[u8]>> ReadCString for Cursor<T> {}

#[derive(Debug)]
pub struct A2SPlayer {
    pub index: u8,
    pub name: CString,
    pub score: i32,
    pub duration: f32,
}

#[derive(Debug)]
pub struct ModData {
    pub link: CString,
    pub download_link: CString,
    _nul: u8,
    pub version: i32,
    pub size: i32,
    pub mp_only: bool,
    pub original_dll: bool,
}

#[derive(Debug)]
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
    pub is_visible: bool,
    pub mod_data: Option<ModData>,
    pub vac_secured: bool,
    pub bots_num: u8,
}

impl A2SInfoOld {
    fn read_from(cursor: &mut Cursor<Vec<u8>>) -> IOResult<A2SInfoOld> {
        Ok(A2SInfoOld {
            address: cursor.read_cstring()?,
            name: cursor.read_cstring()?,
            map: cursor.read_cstring()?,
            folder: cursor.read_cstring()?,
            game: cursor.read_cstring()?,
            players: cursor.read_u8()?,
            max_players: cursor.read_u8()?,
            protocol: cursor.read_u8()?,
            server_type: cursor.read_u8()?,
            enviroment: cursor.read_u8()?,
            is_visible: cursor.read_u8()? == 0,
            mod_data: if cursor.read_u8()? == 1 {
                Some(ModData {
                    link: cursor.read_cstring()?,
                    download_link: cursor.read_cstring()?,
                    _nul: cursor.read_u8()?,
                    version: cursor.read_i32::<LittleEndian>()?,
                    size: cursor.read_i32::<LittleEndian>()?,
                    mp_only: cursor.read_u8()? == 1,
                    original_dll: cursor.read_u8()? == 0,
                })
            } else {
                None
            },
            vac_secured: cursor.read_u8()? == 1,
            bots_num: cursor.read_u8()?,
        })
    }
}

#[derive(Debug)]
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
    pub port: Option<i16>,
    pub server_steamid: Option<u64>,
    pub port_source_tv: Option<i16>,
    pub name_source_tv: Option<CString>,
    pub keywords: Option<CString>,
    pub gameid: Option<u64>,
}

impl A2SInfoNew {
    fn read_from(cursor: &mut Cursor<Vec<u8>>) -> IOResult<A2SInfoNew> {
        let protocol = cursor.read_u8()?;
        let name = cursor.read_cstring()?;
        let map = cursor.read_cstring()?;
        let folder = cursor.read_cstring()?;
        let game = cursor.read_cstring()?;
        let steamid = cursor.read_i16::<LittleEndian>()?;
        let players = cursor.read_u8()?;
        let max_players = cursor.read_u8()?;
        let bots = cursor.read_u8()?;
        let server_type = cursor.read_u8()?;
        let enviroment = cursor.read_u8()?;
        let visibility = cursor.read_u8()?;
        let vac = cursor.read_u8()?;
        let version = cursor.read_cstring()?;
        let edf = cursor.read_u8()?;
        let port = if edf & 080 == 1 {
            Some(cursor.read_i16::<LittleEndian>()?)
        } else {
            None
        };
        let server_steamid = if edf & 0x10 == 1 {
            Some(cursor.read_u64::<LittleEndian>()?)
        } else {
            None
        };
        let (port_source_tv, name_source_tv) = if edf & 0x40 == 1 {
            (
                Some(cursor.read_i16::<LittleEndian>()?),
                Some(cursor.read_cstring()?),
            )
        } else {
            (None, None)
        };
        let keywords = if edf & 0x20 == 1 {
            Some(cursor.read_cstring()?)
        } else {
            None
        };
        let gameid = if edf & 0x01 == 1 {
            Some(cursor.read_u64::<LittleEndian>()?)
        } else {
            None
        };

        Ok(A2SInfoNew {
            protocol,
            name,
            map,
            folder,
            game,
            steamid,
            players,
            max_players,
            bots,
            server_type,
            enviroment,
            is_visible: visibility == 0,
            vac_secured: vac == 1,
            version,
            port,
            server_steamid,
            port_source_tv,
            name_source_tv,
            keywords,
            gameid,
        })
    }
}

#[derive(Debug)]
pub enum QueryError {
    IOErr(IOError),
    UnknownHeader(u8),
}

pub type QueryResult<T> = Result<T, QueryError>;

impl From<IOError> for QueryError {
    fn from(err: IOError) -> QueryError {
        QueryError::IOErr(err)
    }
}

// TODO : impl Error for QueryError

pub struct ValveQuery(UdpSocket);

impl ValveQuery {
    pub fn bind(addr: SocketAddr) -> IOResult<ValveQuery> {
        Ok(ValveQuery(UdpSocket::bind(addr)?))
    }

    pub fn connect(&self, addr: SocketAddr) -> IOResult<()> {
        self.0.connect(addr)
    }

    pub fn timeout(&self) -> IOResult<Option<Duration>> {
        self.0.read_timeout()
    }

    pub fn set_timeout(&self, timeout: Option<Duration>) -> IOResult<()> {
        self.0.set_read_timeout(timeout)
    }

    fn read_raw(&self) -> IOResult<Vec<u8>> {
        let mut buf = [0; PACKET_SIZE];
        let size = self.0.recv(&mut buf)?;
        Ok(buf[..size].to_vec())
    }

    fn read(&self) -> IOResult<Vec<u8>> {
        let mut packets: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut num = 1;
        let mut unique_id = 0;

        while packets.len() < num {
            let raw_packet = self.read_raw()?;
            if let Some(packet) = Packet::parse(raw_packet)? {
                if packets.len() == 0 {
                    // First packet is base of id and num data
                    unique_id = packet.unique_id;
                    num = packet.packets_num;
                } else if unique_id != packet.unique_id || num != packet.packets_num {
                    continue; // skip wrong packets to catch another one
                }
                packets.push((packet.id, packet.data));
            }
        }

        packets.sort_by(|(id1, _), (id2, _)| id1.cmp(id2));
        Ok(packets.into_iter().flat_map(|(_, data)| data).collect())
    }

    fn request(&self, buf: &[u8]) -> IOResult<Vec<u8>> {
        self.0.send(buf)?;
        self.read()
    }

    pub fn a2s_info(&self) -> QueryResult<Either<A2SInfoOld, A2SInfoNew>> {
        let data: &'static [u8] = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00";
        let answer = self.request(data)?;
        let mut cursor = Cursor::new(answer);
        let header = cursor.read_u8()?;
        match header {
            b'm' => Ok(Either::Left(A2SInfoOld::read_from(&mut cursor)?)),
            b'I' => Ok(Either::Right(A2SInfoNew::read_from(&mut cursor)?)),
            _ => Err(QueryError::UnknownHeader(header)),
        }
    }

    fn a2s_challenge(&self, data: &'static [u8]) -> QueryResult<i32> {
        let answer = self.request(data)?;
        let mut cursor = Cursor::new(answer);
        let header = cursor.read_u8()?;
        match header {
            b'A' => Ok(cursor.read_i32::<LittleEndian>()?),
            _ => Err(QueryError::UnknownHeader(header)),
        }
    }

    pub fn a2s_player_challenge(&self) -> QueryResult<i32> {
        self.a2s_challenge(b"\xFF\xFF\xFF\xFFU\xFF\xFF\xFF\xFF")
    }

    pub fn a2s_rules_challenge(&self) -> QueryResult<i32> {
        self.a2s_challenge(b"\xFF\xFF\xFF\xFFV\xFF\xFF\xFF\xFF")
    }

    pub fn a2s_player(&self, challenge: i32) -> QueryResult<Vec<A2SPlayer>> {
        let mut data = [0xFF, 0xFF, 0xFF, 0xFF, b'U', 0x0, 0x0, 0x0, 0x0];
        LittleEndian::write_i32(&mut data[5..9], challenge);
        let answer = self.request(&data)?;
        let mut cursor = Cursor::new(answer);
        let header = cursor.read_u8()?;
        if header != b'D' {
            return Err(QueryError::UnknownHeader(header));
        }
        let players_num = cursor.read_u8()?;
        let mut players: Vec<A2SPlayer> = Vec::with_capacity(players_num as usize);
        for _ in 0..players_num {
            players.push(A2SPlayer {
                index: cursor.read_u8()?,
                name: cursor.read_cstring()?,
                score: cursor.read_i32::<LittleEndian>()?,
                duration: cursor.read_f32::<LittleEndian>()?,
            });
        }
        Ok(players)
    }

    pub fn a2s_rules(&self, challenge: i32) -> QueryResult<HashMap<CString, CString>> {
        let mut data = [0xFF, 0xFF, 0xFF, 0xFF, b'V', 0x0, 0x0, 0x0, 0x0];
        LittleEndian::write_i32(&mut data[5..9], challenge);
        let answer = self.request(&data)?;
        let mut cursor = Cursor::new(answer);

        let header = cursor.read_u8()?;
        if header != b'E' {
            return Err(QueryError::UnknownHeader(header));
        }
        let _ = cursor.read_i16::<LittleEndian>()?; // this may be wrong so don't use it
        let strs = cursor
            .split(b'\0')
            .filter_map(|e| match e {
                Ok(s) => match CString::new(s) {
                    Ok(cstr) => Some(cstr),
                    Err(_) => None, // TODO : not ignore wrong strings but return an error
                },
                Err(_) => None,
            })
            .tuples::<(_, _)>()
            .collect::<HashMap<_, _>>();

        Ok(strs)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const ADDR: &'static str = "62.140.250.10:27015";

    #[test]
    fn a2s_connection() {
        let query = ValveQuery::bind("0.0.0.0:27315".parse().unwrap()).unwrap();
        query.set_timeout(Some(Duration::new(10, 0))).unwrap();
        query.connect(ADDR.parse().unwrap()).unwrap();
    }

    #[test]
    fn a2s_info_test() {
        let query = ValveQuery::bind("0.0.0.0:27415".parse().unwrap()).unwrap();
        query.set_timeout(Some(Duration::new(10, 0))).unwrap();
        query.connect(ADDR.parse().unwrap()).unwrap();
        println!("{:?}", query.a2s_info().unwrap());
    }

    #[test]
    fn a2s_player_test() {
        let query = ValveQuery::bind("0.0.0.0:27515".parse().unwrap()).unwrap();
        query.set_timeout(Some(Duration::new(10, 0))).unwrap();
        query.connect(ADDR.parse().unwrap()).unwrap();
        let challenge = query.a2s_player_challenge().unwrap();
        let answer = query.a2s_player(challenge).unwrap();
        println!("{}", challenge);
        println!("{:?}", answer);
    }

    #[test]
    fn a2s_rules_test() {
        let query = ValveQuery::bind("0.0.0.0:27615".parse().unwrap()).unwrap();
        query.set_timeout(Some(Duration::new(10, 0))).unwrap();
        query.connect(ADDR.parse().unwrap()).unwrap();
        let challenge = query.a2s_rules_challenge().unwrap();
        let answer = query.a2s_rules(challenge).unwrap();
        println!("{}", challenge);
        println!("{:?}", answer);
    }
}
