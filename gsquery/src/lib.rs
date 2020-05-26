extern crate byteorder;

use byteorder::{ByteOrder, LittleEndian};
use std::{
    io,
    net::{SocketAddr, UdpSocket},
};

const PACKET_SIZE: usize = 1400;

type IOResult<T> = io::Result<T>;

struct Packet {
    header: i32,
    id: i32,
    packet_id: usize,
    packet_num: usize,
    data: Vec<u8>,
}

impl Packet {
    fn parse(packet: Vec<u8>) -> Packet {
        // TODO : more error processing
        let header = LittleEndian::read_i32(&packet[0..4]);
        match header {
            -1 => Packet {
                header,
                id: 0,
                packet_id: 0,
                packet_num: 1,
                data: packet[4..].to_vec(),
            },
            -2 => Packet {
                header,
                id: LittleEndian::read_i32(&packet[4..8]),
                packet_id: (packet[8] & 0xF0 >> 4) as usize,
                packet_num: (packet[8] & 0xF0) as usize,
                data: packet[9..].to_vec(),
            },
            _ => unimplemented!(), // TODO : Result
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

    fn read_whole(&self) -> IOResult<Vec<u8>> {
        let mut buf = [0; PACKET_SIZE];
        let size = self.0.recv(&mut buf)?;
        Ok(buf[..size].to_vec())
    }

    fn read(&self) -> IOResult<Vec<u8>> {
        let mut packets: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut base_num = 1;
        let mut base_id = 0;

        while packets.len() < base_num {
            let packet = self.read_whole()?;
            let Packet {
                header,
                id,
                packet_id,
                packet_num,
                data,
            } = Packet::parse(packet);
            if packets.len() == 0 {
                // First packet is base of id and num data
                base_id = id;
                base_num = packet_num;
            } else if base_id != id || base_num != packet_num || header != -2 {
                unimplemented!();
            }
            packets.push((packet_id, data));
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
