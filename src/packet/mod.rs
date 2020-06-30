use bzip2::{Decompress, Error as Bz2Error};
use crc::crc32::checksum_ieee;
use nom_derive::Nom;
use std::{io::Result as IOResult, net::UdpSocket};

pub mod error;
use error::{Error as PacketError, MultiHeader, PacketResult};

const DEFAULT_PACKET_SIZE: usize = 1400;

struct DecompressInfo {
    decompressed_size: u32,
    crc32_sum: u32,
}

pub struct MultiPacket {
    uid: u32,
    index: usize,
    total: usize,
    switch_size: usize, // TODO : write comments
    decompress_info: Option<DecompressInfo>,
    payload: Vec<u8>,
}

pub trait PacketParser {
    fn parse(i: &[u8]) -> nom::IResult<&[u8], MultiPacket>;
}

pub struct GoldsrcParser;

impl PacketParser for GoldsrcParser {
    fn parse(i: &[u8]) -> nom::IResult<&[u8], MultiPacket> {
        let (i, uid) = nom::number::streaming::le_u32(i)?;
        let (i, num) = nom::number::streaming::le_u8(i)?;
        Ok((
            &[],
            MultiPacket {
                uid,
                index: ((num & 0xF0) >> 4) as usize,
                total: (num & 0xF0) as usize,
                switch_size: DEFAULT_PACKET_SIZE,
                decompress_info: None,
                payload: i.to_vec(),
            },
        ))
    }
}

pub struct SourceParser;

impl PacketParser for SourceParser {
    fn parse(i: &[u8]) -> nom::IResult<&[u8], MultiPacket> {
        #[derive(Nom)]
        #[nom(LittleEndian)]
        struct SourcePacket {
            uid: u32,
            total: u8,
            index: u8,
            size: u16,
            #[nom(Cond = "uid & 0x80000000 != 0")]
            decomp_data: Option<u32>,
            #[nom(Cond = "uid & 0x80000000 != 0")]
            crc32: Option<u32>,
        }
        let (i, packet) = SourcePacket::parse(i)?;
        Ok((
            &[],
            MultiPacket {
                uid: packet.uid,
                index: packet.index as usize,
                total: packet.total as usize,
                switch_size: packet.size as usize,
                decompress_info: if let (Some(decompressed_size), Some(crc32_sum)) =
                    (packet.decomp_data, packet.crc32)
                {
                    Some(DecompressInfo {
                        crc32_sum,
                        decompressed_size,
                    })
                } else {
                    None
                },
                payload: i.to_vec(),
            },
        ))
    }
}

fn read_raw(socket: &UdpSocket, packet_size: usize) -> IOResult<Vec<u8>> {
    let mut buf = vec![0; packet_size];
    let size = socket.recv(&mut buf)?;
    buf.truncate(size);
    Ok(buf)
}

fn decompress(compressed: Vec<u8>, output_size: usize) -> Result<Vec<u8>, Bz2Error> {
    let mut decompressed = vec![0; output_size];
    let mut decompressor = Decompress::new(false); // Packets won't be large anyway, so don't worry about memmory
    decompressor.decompress(&compressed, &mut decompressed)?;
    Ok(decompressed)
}

fn read_multi<P: PacketParser>(i: &[u8], socket: &UdpSocket) -> PacketResult<Vec<u8>> {
    let (_, init_packet) = P::parse(i)?;

    let mut payloads: Vec<Vec<u8>> = vec![vec![]; init_packet.total];
    payloads.insert(init_packet.index, init_packet.payload);
    while payloads.len() < payloads.capacity() {
        let packet = read_raw(socket, init_packet.switch_size)?;
        let (i, header) = nom::number::streaming::le_i32(&packet)?;
        if header != -2 {
            return Err(PacketError::WrongHeader(header));
        }
        let (_, new_packet) = P::parse(i)?;
        if init_packet.uid != new_packet.uid || init_packet.total != new_packet.total {
            return Err(PacketError::Interrupted {
                base: MultiHeader {
                    uid: init_packet.uid,
                    total: init_packet.total,
                },
                wrong: MultiHeader {
                    uid: new_packet.uid,
                    total: new_packet.total,
                },
            });
        }
        payloads.insert(new_packet.index as usize, new_packet.payload);
    }

    let full_payload = payloads.into_iter().flatten().collect();
    if let Some(decompress_info) = init_packet.decompress_info {
        let full_payload = decompress(full_payload, decompress_info.decompressed_size as usize)?;
        let expected_crc32 = decompress_info.crc32_sum;
        let calculated_crc32 = checksum_ieee(&full_payload);
        if expected_crc32 != calculated_crc32 {
            return Err(PacketError::Crc32(expected_crc32, calculated_crc32));
        }
        return Ok(full_payload);
    }

    Ok(full_payload)
}

pub(crate) fn read_payload<P: PacketParser>(socket: &UdpSocket) -> PacketResult<Vec<u8>> {
    let packet = read_raw(socket, DEFAULT_PACKET_SIZE)?;
    let (i, header) = nom::number::streaming::le_i32(&packet)?;
    match header {
        -1 => Ok(i.to_vec()),
        -2 => read_multi::<P>(i, socket),
        _ => Err(PacketError::WrongHeader(header)),
    }
}
