use byteorder::{ByteOrder, ReadBytesExt, LE};
use byteorder_parser::*;
use either::Either;
use std::{
    collections::HashMap,
    ffi::CString,
    io::{BufRead, Error as IOError, ErrorKind, Read, Result as IOResult},
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

mod types;
pub use types::*;

mod error;
pub use error::*;

const PACKET_SIZE: usize = 1400;

struct Packet {
    unique_id: i32,
    id: usize,
    packets_num: usize,
    data: Vec<u8>,
}

impl Packet {
    fn parse(packet: Vec<u8>) -> IOResult<Option<Packet>> {
        let mut buf = packet.as_slice();
        let header = buf.read_i32::<LE>()?;
        match header {
            -1 => {
                let mut data = Vec::new();
                buf.read_to_end(&mut data)?;
                Ok(Some(Packet {
                    unique_id: 0,
                    id: 0,
                    packets_num: 1,
                    data,
                }))
            }
            -2 => {
                let id = buf.read_i32::<LE>()?;
                let num = buf.read_u8()?;
                let mut data = Vec::new();
                buf.read_to_end(&mut data)?;
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
                if packets.is_empty() {
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
        let mut buf = answer.as_slice();
        let header = buf.read_u8()?;
        match header {
            b'm' => Ok(Either::Left(A2SInfoOld::read_with_byteorder::<LE, _>(
                &mut buf,
            )?)),
            b'I' => Ok(Either::Right(A2SInfoNew::read_with_byteorder::<LE, _>(
                &mut buf,
            )?)),
            _ => Err(QueryError::UnknownHeader(header, "109 or 073")),
        }
    }

    fn a2s_challenge(&self, data: &'static [u8]) -> QueryResult<i32> {
        let answer = self.request(data)?;
        let mut buf = answer.as_slice();
        let header = buf.read_u8()?;
        match header {
            b'A' => Ok(buf.read_i32::<LE>()?),
            _ => Err(QueryError::UnknownHeader(header, "065")),
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
        LE::write_i32(&mut data[5..9], challenge);
        let answer = self.request(&data)?;
        let mut buf = answer.as_slice();
        let header = buf.read_u8()?;
        match header {
            b'D' => {
                let players_num = buf.read_u8()?;
                let mut players: Vec<A2SPlayer> = Vec::with_capacity(players_num as usize);
                for _ in 0..players_num {
                    players.push(A2SPlayer::read_with_byteorder::<LE, _>(&mut buf)?);
                }
                Ok(players)
            }
            _ => Err(QueryError::UnknownHeader(header, "068")),
        }
    }

    pub fn a2s_rules(&self, challenge: i32) -> QueryResult<HashMap<CString, CString>> {
        let mut data = [0xFF, 0xFF, 0xFF, 0xFF, b'V', 0x0, 0x0, 0x0, 0x0];
        LE::write_i32(&mut data[5..9], challenge);
        let answer = self.request(&data)?;
        let mut buf = answer.as_slice();

        let header = buf.read_u8()?;
        match header {
            b'E' => {
                let num = buf.read_i16::<LE>()?;
                let mut strs = BufRead::split(buf, b'\0').map(|res| match res {
                    Ok(bytes) => Ok(CString::new(bytes)?),
                    Err(e) => Err(IOError::new(ErrorKind::InvalidData, e)),
                });
                let mut out = HashMap::<CString, CString>::with_capacity(num as usize);
                while let (Some(s1), Some(s2)) = (strs.next(), strs.next()) {
                    out.insert(s1?, s2?);
                }
                Ok(out)
            }
            _ => Err(QueryError::UnknownHeader(header, "069")),
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
