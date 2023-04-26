use std::io;

use bytes::{Buf, BytesMut};
use bzip2::Decompress;
use crc::{Crc, CRC_32_CKSUM};

use super::DEFAULT_FRAME_SIZE;

pub trait PacketHeader: Sized {
    fn read_header(buf: &mut BytesMut) -> io::Result<Self>;

    fn index(&self) -> usize;
    fn total(&self) -> usize;
    fn switch_size(&self) -> usize;

    fn post_process(self, input: BytesMut) -> io::Result<BytesMut>;
}

pub struct GoldsrcHeader {
    num: u8,
}

impl PacketHeader for GoldsrcHeader {
    fn read_header(buf: &mut BytesMut) -> io::Result<Self> {
        if buf.len() < 5 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not enough bytes for goldsrc multi packet",
            ));
        }

        let _uid = buf.get_i32_le();
        let num = buf.get_u8();

        Ok(Self { num })
    }

    fn index(&self) -> usize {
        (self.num >> 4) as usize
    }

    fn total(&self) -> usize {
        (self.num & 0xF) as usize
    }

    fn switch_size(&self) -> usize {
        DEFAULT_FRAME_SIZE
    }

    fn post_process(self, input: BytesMut) -> io::Result<BytesMut> {
        Ok(input)
    }
}

pub struct SourceHeader {
    total: u8,
    index: u8,
    packet_size: u16,
    decomp_info: Option<(u32, u32)>,
}

impl PacketHeader for SourceHeader {
    fn read_header(buf: &mut BytesMut) -> io::Result<Self> {
        if buf.len() < 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not enough bytes for source multi packet",
            ));
        }

        let uid = buf.get_u32_le();
        let total = buf.get_u8();
        let index = buf.get_u8();
        let packet_size = buf.get_u16_le();
        let decomp_info = if uid & 0x80000000 != 0 {
            if buf.len() < 8 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "not enough bytes for decompression info",
                ));
            }
            Some((buf.get_u32_le(), buf.get_u32_le()))
        } else {
            None
        };

        Ok(Self {
            total,
            index,
            packet_size,
            decomp_info,
        })
    }

    fn index(&self) -> usize {
        self.index as usize
    }

    fn total(&self) -> usize {
        self.total as usize
    }

    fn switch_size(&self) -> usize {
        self.packet_size as usize
    }

    fn post_process(self, input: BytesMut) -> io::Result<BytesMut> {
        if let Some((decompression_size, crc32)) = self.decomp_info {
            let mut out = BytesMut::zeroed(decompression_size as usize);
            let mut decompressor = Decompress::new(false);
            decompressor.decompress(input.as_ref(), out.as_mut())?;

            let crc = Crc::<u32>::new(&CRC_32_CKSUM);
            if crc.checksum(&out) == crc32 {
                Ok(out)
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "wrong crc32"))
            }
        } else {
            Ok(input)
        }
    }
}
