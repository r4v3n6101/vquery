use std::io::Result as IOResult;
use std::net::UdpSocket;

const PACKET_SIZE: usize = 1400;

#[derive(Debug)]
struct GoldsrcPacket {
    header: i32,
    uid: i32,
    index: u8,
    packets_num: u8,
}

impl GoldsrcPacket {
    fn parse(i: &[u8]) -> nom::IResult<&[u8], GoldsrcPacket> {
        let (i, header) =
            nom::combinator::verify(nom::number::streaming::le_i32, |&x| x == -1 || x == -2)(i)?;
        let (i, uid, num) = match header {
            -1 => (i, 0, 1),
            -2 => {
                let (i, uid) = nom::number::streaming::le_i32(i)?;
                let (i, num) = nom::number::streaming::le_u8(i)?;
                (i, uid, num)
            }
            _ => unreachable!(), // checked above
        };
        let index = (num & 0xF0) >> 4;
        let packets_num = num & 0xF0;
        Ok((
            i,
            GoldsrcPacket {
                header,
                uid,
                index,
                packets_num,
            },
        ))
    }
}

pub(crate) fn read(socket: &UdpSocket) -> IOResult<Vec<u8>> {
    let mut packets: Vec<(usize, Vec<u8>)> = Vec::new();
    let mut num = 1;
    let mut unique_id = 0;

    while packets.len() < num {
        let mut buf = [0; PACKET_SIZE];
        let size = socket.recv(&mut buf)?;
        let raw_packet = &buf[..size];

        if let Ok((i, packet)) = GoldsrcPacket::parse(raw_packet) {
            if packets.is_empty() {
                // First packet is base of id and num data
                unique_id = packet.uid;
                num = packet.packets_num as usize;
            } else if unique_id != packet.uid || num != packet.packets_num as usize {
                continue; // skip wrong packets to catch another one
            }
            packets.push((packet.index as usize, i.to_vec()));
        }
    }

    packets.sort_by(|(id1, _), (id2, _)| id1.cmp(id2));
    Ok(packets.into_iter().flat_map(|(_, i)| i).collect()) // TODO : temporary solution
}
