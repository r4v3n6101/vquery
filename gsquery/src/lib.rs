extern crate byteorder;
extern crate itertools;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use itertools::Itertools;
use std::{
    collections::HashMap,
    ffi::CString,
    io::{BufRead, Cursor, Read, Result as IOResult},
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

    pub fn a2s_info(&self) -> IOResult<Vec<u8>> {
        let data: &'static [u8] = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00";
        let answer = self.request(data)?;

        Ok(answer)
    }

    fn a2s_challenge(&self, data: &'static [u8]) -> IOResult<Option<i32>> {
        let answer = self.request(data)?;
        let mut cursor = Cursor::new(answer);
        let header = cursor.read_u8()?;
        match header {
            b'A' => Ok(Some(cursor.read_i32::<LittleEndian>()?)),
            _ => Ok(None),
        }
    }

    pub fn a2s_player_challenge(&self) -> IOResult<Option<i32>> {
        self.a2s_challenge(b"\xFF\xFF\xFF\xFFU\xFF\xFF\xFF\xFF")
    }

    pub fn a2s_rules_challenge(&self) -> IOResult<Option<i32>> {
        self.a2s_challenge(b"\xFF\xFF\xFF\xFFV\xFF\xFF\xFF\xFF")
    }

    pub fn a2s_player(&self, challenge: i32) -> IOResult<Option<Vec<u8>>> {
        let mut data = [0xFF, 0xFF, 0xFF, 0xFF, b'U', 0x0, 0x0, 0x0, 0x0];
        LittleEndian::write_i32(&mut data[5..9], challenge);
        let answer = self.request(&data)?;
        Ok(Some(answer))
    }

    pub fn a2s_rules(&self, challenge: i32) -> IOResult<Option<HashMap<CString, CString>>> {
        let mut data = [0xFF, 0xFF, 0xFF, 0xFF, b'V', 0x0, 0x0, 0x0, 0x0];
        LittleEndian::write_i32(&mut data[5..9], challenge);
        let answer = self.request(&data)?;
        let mut cursor = Cursor::new(answer);

        let header = cursor.read_u8()?;
        if header != b'E' {
            return Ok(None);
        }
        let _ = cursor.read_i16::<LittleEndian>()?; // this may be wrong so don't use it
        let strs = cursor
            .split(b'\0')
            .filter_map(|e| match e {
                Ok(s) => match CString::new(s) {
                    Ok(cstr) => Some(cstr),
                    Err(_) => None, // TODO : not ignore wrong strings but threw an error
                },
                Err(_) => None,
            })
            .tuples::<(_, _)>()
            .collect::<HashMap<_, _>>();

        if strs.is_empty() {
            return Ok(None);
        }

        Ok(Some(strs))
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
        let challenge = query.a2s_player_challenge().unwrap().unwrap();
        let answer = query.a2s_player(challenge).unwrap().unwrap();
        println!("{}", challenge);
        println!("{:?}", answer);
    }

    #[test]
    fn a2s_rules_test() {
        let query = ValveQuery::bind("0.0.0.0:27615".parse().unwrap()).unwrap();
        query.set_timeout(Some(Duration::new(10, 0))).unwrap();
        query.connect(ADDR.parse().unwrap()).unwrap();
        let challenge = query.a2s_rules_challenge().unwrap().unwrap();
        let answer = query.a2s_rules(challenge).unwrap().unwrap();
        println!("{}", challenge);
        println!("{:?}", answer);
    }
}
