use nom_derive::Nom;
use std::{
    io::Result as IOResult,
    marker::PhantomData,
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

// TODO : visibility
mod packet;
use packet::read_payload;
pub use packet::{GoldsrcParser, PacketParser, SourceParser};

mod error;
pub use error::*;

mod a2s;
pub use a2s::*;

pub struct ValveQuery<P: PacketParser>(UdpSocket, PhantomData<P>);

impl<P: PacketParser> ValveQuery<P> {
    pub fn bind(addr: SocketAddr) -> IOResult<Self> {
        Ok(Self(UdpSocket::bind(addr)?, PhantomData))
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

    fn request(&self, buf: &[u8]) -> QueryResult<Vec<u8>> {
        self.0.send(buf).map_err(packet::error::Error::from)?;
        Ok(read_payload::<P>(&self.0)?)
    }

    fn a2s_challenge(&self, data: &'static [u8]) -> QueryResult<u32> {
        #[derive(Nom)]
        #[nom(LittleEndian)]
        struct A2SChallenge<'a> {
            #[nom(Tag(b"A"))]
            _header: &'a [u8],
            challenge: u32,
        }
        let answer = self.request(data)?;
        let (_, a2s_challenge) = A2SChallenge::parse(&answer)?;
        Ok(a2s_challenge.challenge)
    }

    pub fn a2s_player_challenge(&self) -> QueryResult<u32> {
        self.a2s_challenge(b"\xFF\xFF\xFF\xFFU\xFF\xFF\xFF\xFF")
    }

    pub fn a2s_rules_challenge(&self) -> QueryResult<u32> {
        self.a2s_challenge(b"\xFF\xFF\xFF\xFFV\xFF\xFF\xFF\xFF")
    }

    pub fn a2s_info_old(&self) -> QueryResult<InfoOld> {
        #[derive(Nom)]
        #[nom(LittleEndian)]
        struct A2SInfoOld<'a> {
            #[nom(Tag(b"m"))]
            _header: &'a [u8],
            info: InfoOld,
        }

        let answer = self.request(b"\xFF\xFF\xFF\xFFTSource Engine Query\x00")?;
        let (_, a2s_info_old) = A2SInfoOld::parse(&answer)?;
        Ok(a2s_info_old.info)
    }

    pub fn a2s_info_new(&self) -> QueryResult<InfoNew> {
        #[derive(Nom)]
        #[nom(LittleEndian)]
        struct A2SInfoNew<'a> {
            #[nom(Tag(b"I"))]
            _header: &'a [u8],
            info: InfoNew,
        }

        let answer = self.request(b"\xFF\xFF\xFF\xFFTSource Engine Query\x00")?;
        let (_, a2s_info_new) = A2SInfoNew::parse(&answer)?;
        Ok(a2s_info_new.info)
    }

    pub fn a2s_players(&self, challenge: u32) -> QueryResult<PlayersList> {
        #[derive(Nom)]
        #[nom(LittleEndian)]
        struct A2SPlayer<'a> {
            #[nom(Tag(b"D"))]
            _header: &'a [u8],
            list: PlayersList,
        }
        let challenge = challenge.to_le_bytes();
        let data = [
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            b'U',
            challenge[0],
            challenge[1],
            challenge[2],
            challenge[3],
        ];
        let answer = self.request(&data)?;
        let (_, a2s_player) = A2SPlayer::parse(&answer)?;
        Ok(a2s_player.list)
    }

    pub fn a2s_rules(&self, challenge: u32) -> QueryResult<RulesList> {
        #[derive(Nom)]
        #[nom(LittleEndian)]
        struct A2SRules<'a> {
            #[nom(Tag(b"E"))]
            _header: &'a [u8],
            list: RulesList,
        }
        let challenge = challenge.to_le_bytes();
        let data = [
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            b'V',
            challenge[0],
            challenge[1],
            challenge[2],
            challenge[3],
        ];
        let answer = self.request(&data)?;

        let mut slice = answer.as_slice();
        if let Ok((i, four_ff)) =
            nom::number::complete::le_u32::<_, (_, nom::error::ErrorKind)>(slice)
        {
            if four_ff == 0xFFFF_FFFF {
                // Undocumented: a2s_rules may start with four FF before header 0x45 (it's not single packet marker)
                slice = i;
            }
        }

        let (_, a2s_rules) = A2SRules::parse(slice)?;
        Ok(a2s_rules.list)
    }
}
