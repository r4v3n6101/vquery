use std::{collections::BTreeMap, io, marker::PhantomData, mem};

use bytes::{Buf, Bytes, BytesMut};
use tokio_util::codec::Decoder;

mod header;

const DEFAULT_FRAME_SIZE: usize = 1400;

pub type GoldsrcDecoder<I> = QueryDecoder<I, header::GoldsrcHeader>;
pub type SourceDecoder<I> = QueryDecoder<I, header::SourceHeader>;

pub struct QueryDecoder<I, P> {
    frame_len: usize,
    buffer: BTreeMap<usize, Bytes>,

    inner: I,

    _phantom: PhantomData<P>,
}

impl<I, P> QueryDecoder<I, P> {
    pub fn new(inner: I) -> Self {
        Self {
            frame_len: DEFAULT_FRAME_SIZE,
            buffer: Default::default(),

            inner,
            _phantom: PhantomData,
        }
    }
}

impl<I, P> Decoder for QueryDecoder<I, P>
where
    I: Decoder,
    P: header::PacketHeader,
{
    type Item = I::Item;
    type Error = I::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            // Take more if not enough bytes in frame
            if src.len() < self.frame_len {
                return Ok(None);
            }

            let header = src.get_i32_le();
            let packet_len = self.frame_len - 4;
            match header {
                -1 => return self.inner.decode(&mut src.split_to(packet_len)),
                -2 => {
                    let multi_len = src.len();
                    let pheader = P::read_header(src)?;
                    let header_len = multi_len - src.len();
                    let payload = src.split_to(packet_len - header_len).freeze();
                    self.buffer.insert(pheader.index(), payload);

                    if self.buffer.len() == pheader.total() {
                        // Reset to default packet size
                        self.frame_len = DEFAULT_FRAME_SIZE;
                        let intermediate: BytesMut = mem::take(&mut self.buffer)
                            .into_iter()
                            .flat_map(|(_, buf)| buf)
                            .collect();
                        let mut out = pheader.post_process(intermediate)?;

                        return self.inner.decode(&mut out);
                    } else {
                        // All next packages will use new size
                        self.frame_len = pheader.switch_size();
                        if self.frame_len < 4 {
                            self.frame_len = DEFAULT_FRAME_SIZE;
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "malformed frame_len field",
                            )
                            .into());
                        }

                        continue;
                    }
                }
                _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "wrong header").into()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use bytes::{BufMut, BytesMut};
    use futures::StreamExt;
    use tokio_util::codec::{BytesCodec, FramedRead};

    use crate::codec::GoldsrcDecoder;

    use super::DEFAULT_FRAME_SIZE;

    #[tokio::test]
    async fn test_goldsrc_single_packet() {
        let mut bytes = BytesMut::with_capacity(DEFAULT_FRAME_SIZE);
        bytes.put_i32_le(-1);
        bytes.put_bytes(0x11, DEFAULT_FRAME_SIZE - 4);

        let bytes = bytes.freeze();
        let payload = bytes.slice(4..);

        let input = &mut bytes.as_ref();
        let mut framed_reader = FramedRead::new(input, GoldsrcDecoder::new(BytesCodec::new()));

        assert_eq!(framed_reader.next().await.unwrap().unwrap(), payload);
    }

    #[tokio::test]
    async fn test_goldsrc_multi_packet() {
        let mut first = BytesMut::with_capacity(DEFAULT_FRAME_SIZE);
        first.put_i32_le(-2);
        first.put_i32_le(20);
        first.put_u8(2 << 4 | 3);
        first.put_bytes(0x11, DEFAULT_FRAME_SIZE - 9);

        let mut second = BytesMut::with_capacity(DEFAULT_FRAME_SIZE);
        second.put_i32_le(-2);
        second.put_i32_le(20);
        second.put_u8(1 << 4 | 3);
        second.put_bytes(0x12, DEFAULT_FRAME_SIZE - 9);

        let mut third = BytesMut::with_capacity(DEFAULT_FRAME_SIZE);
        third.put_i32_le(-2);
        third.put_i32_le(20);
        third.put_u8(3 << 4 | 3);
        third.put_bytes(0x13, DEFAULT_FRAME_SIZE - 9);

        let input = Cursor::new(
            [first, second, third]
                .iter()
                .flat_map(|buf| buf)
                .copied()
                .collect::<Vec<_>>(),
        );
        let mut framed_reader = FramedRead::new(input, GoldsrcDecoder::new(BytesCodec::new()));

        assert_eq!(
            framed_reader.next().await.unwrap().unwrap().to_vec(),
            [
                vec![0x12; DEFAULT_FRAME_SIZE - 9],
                vec![0x11; DEFAULT_FRAME_SIZE - 9],
                vec![0x13; DEFAULT_FRAME_SIZE - 9]
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
        );
    }
}
