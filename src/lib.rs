extern crate byteorder;
extern crate either;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use either::Either;
use std::{
    collections::HashMap,
    error::Error,
    ffi::{CStr, CString},
    fmt::{Display, Formatter, Result as FmtResult},
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

impl<T: BufRead> ReadCString for T {}

#[derive(Debug)]
pub struct A2SPlayer {
    pub index: u8,
    pub name: CString,
    pub score: i32,
    pub duration: Duration,
}

impl A2SPlayer {
    fn read_from<T: BufRead>(reader: &mut T) -> IOResult<A2SPlayer> {
        Ok(A2SPlayer {
            index: reader.read_u8()?,
            name: reader.read_cstring()?,
            score: reader.read_i32::<LittleEndian>()?,
            duration: Duration::from_secs_f32(reader.read_f32::<LittleEndian>()?),
        })
    }
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

impl ModData {
    fn read_from<T: BufRead>(reader: &mut T) -> IOResult<ModData> {
        Ok(ModData {
            link: reader.read_cstring()?,
            download_link: reader.read_cstring()?,
            _nul: reader.read_u8()?,
            version: reader.read_i32::<LittleEndian>()?,
            size: reader.read_i32::<LittleEndian>()?,
            mp_only: reader.read_u8()? == 1,
            original_dll: reader.read_u8()? == 0,
        })
    }
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
    fn read_from<T: BufRead>(reader: &mut T) -> IOResult<A2SInfoOld> {
        Ok(A2SInfoOld {
            address: reader.read_cstring()?,
            name: reader.read_cstring()?,
            map: reader.read_cstring()?,
            folder: reader.read_cstring()?,
            game: reader.read_cstring()?,
            players: reader.read_u8()?,
            max_players: reader.read_u8()?,
            protocol: reader.read_u8()?,
            server_type: reader.read_u8()?,
            enviroment: reader.read_u8()?,
            is_visible: reader.read_u8()? == 0,
            mod_data: if reader.read_u8()? == 1 {
                Some(ModData::read_from(reader)?)
            } else {
                None
            },
            vac_secured: reader.read_u8()? == 1,
            bots_num: reader.read_u8()?,
        })
    }
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

impl ExtraData {
    fn read_from<T: BufRead>(reader: &mut T) -> IOResult<ExtraData> {
        let edf = reader.read_u8()?;
        Ok(ExtraData {
            port: if edf & 080 == 1 {
                Some(reader.read_i16::<LittleEndian>()?)
            } else {
                None
            },
            server_steamid: if edf & 0x10 == 1 {
                Some(reader.read_u64::<LittleEndian>()?)
            } else {
                None
            },
            port_source_tv: if edf & 0x40 == 1 {
                Some(reader.read_i16::<LittleEndian>()?)
            } else {
                None
            },
            name_source_tv: if edf & 0x40 == 1 {
                Some(reader.read_cstring()?)
            } else {
                None
            },
            keywords: if edf & 0x20 == 1 {
                Some(reader.read_cstring()?)
            } else {
                None
            },
            gameid: if edf & 0x01 == 1 {
                Some(reader.read_u64::<LittleEndian>()?)
            } else {
                None
            },
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
    pub extra_data: ExtraData,
}

impl A2SInfoNew {
    fn read_from<T: BufRead>(reader: &mut T) -> IOResult<A2SInfoNew> {
        Ok(A2SInfoNew {
            protocol: reader.read_u8()?,
            name: reader.read_cstring()?,
            map: reader.read_cstring()?,
            folder: reader.read_cstring()?,
            game: reader.read_cstring()?,
            steamid: reader.read_i16::<LittleEndian>()?,
            players: reader.read_u8()?,
            max_players: reader.read_u8()?,
            bots: reader.read_u8()?,
            server_type: reader.read_u8()?,
            enviroment: reader.read_u8()?,
            is_visible: reader.read_u8()? == 0,
            vac_secured: reader.read_u8()? == 1,
            version: reader.read_cstring()?,
            extra_data: ExtraData::read_from(reader)?,
        })
    }
}

#[derive(Debug)]
pub enum QueryError {
    IOErr(IOError),
    UnknownHeader(u8),
}

impl From<IOError> for QueryError {
    fn from(err: IOError) -> QueryError {
        QueryError::IOErr(err)
    }
}

impl Display for QueryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match *self {
            QueryError::IOErr(ref err) => write!(f, "IO error: {}", err),
            QueryError::UnknownHeader(ref header) => write!(f, "Wrong header: {}", header),
        }
    }
}

impl Error for QueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            QueryError::IOErr(ref err) => Some(err),
            QueryError::UnknownHeader(_) => None,
        }
    }
}

pub type QueryResult<T> = Result<T, QueryError>;

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
        match header {
            b'D' => {
                let players_num = cursor.read_u8()?;
                let mut players: Vec<A2SPlayer> = Vec::with_capacity(players_num as usize);
                for _ in 0..players_num {
                    players.push(A2SPlayer::read_from(&mut cursor)?);
                }
                Ok(players)
            }
            _ => Err(QueryError::UnknownHeader(header)),
        }
    }

    pub fn a2s_rules(&self, challenge: i32) -> QueryResult<HashMap<CString, CString>> {
        let mut data = [0xFF, 0xFF, 0xFF, 0xFF, b'V', 0x0, 0x0, 0x0, 0x0];
        LittleEndian::write_i32(&mut data[5..9], challenge);
        let answer = self.request(&data)?;
        let mut cursor = Cursor::new(answer);

        let header = cursor.read_u8()?;
        match header {
            b'E' => {
                let num = cursor.read_i16::<LittleEndian>()?;
                let mut strs = cursor.split(b'\0').map(|res| match res {
                    Ok(bytes) => {
                        CString::new(bytes).map_err(|e| IOError::new(ErrorKind::InvalidData, e))
                    }
                    Err(e) => Err(IOError::new(ErrorKind::InvalidData, e)),
                });
                let mut out = HashMap::<CString, CString>::with_capacity(num as usize);
                while let (Some(s1), Some(s2)) = (strs.next(), strs.next()) {
                    out.insert(s1?, s2?);
                }
                Ok(out)
            }
            _ => Err(QueryError::UnknownHeader(header)),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const ADDR: &'static str = "62.140.250.10:27015";

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
