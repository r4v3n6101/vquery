extern crate byteorder;

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;
use std::{
    io,
    net::{SocketAddr, UdpSocket},
};

const PACKET_SIZE: usize = 1400;

type IOResult<T> = io::Result<T>;

struct Packet {
    unique_id: i32,
    id: usize,
    packets_num: usize,
    data: Vec<u8>,
}

impl Packet {
    fn parse(packet: Vec<u8>) -> IOResult<Option<Packet>> {
        let mut cursor = std::io::Cursor::new(packet);
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

    pub fn a2s_info(&self) -> IOResult<Vec<u8>> {
        let s = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00"; // TODO : const
        self.0.send(s)?;
        Ok(self.read()?)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn it_works() {
        let query = ValveQuery::bind("0.0.0.0:27315".parse().unwrap()).unwrap();
        query
            .connect("213.238.173.152:27015".parse().unwrap())
            .unwrap();
        println!("{:?}", query.a2s_info().unwrap());
    }
}
