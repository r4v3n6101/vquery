use nom_derive::*;
use std::{
    io::Result as IOResult,
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

mod types;
pub use types::*;

mod error;
pub use error::*;

mod engine;
use engine::*;

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

    fn request(&self, buf: &[u8]) -> IOResult<Vec<u8>> {
        self.0.send(buf)?;
        read(&self.0)
    }

    fn a2s_challenge(&self, data: &'static [u8]) -> QueryResult<u32> {
        #[derive(Nom)]
        #[nom(LittleEndian)]
        struct A2SChallenge<'a> {
            #[nom(Tag(b"A"))]
            header: &'a [u8],
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
            header: &'a [u8],
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
            header: &'a [u8],
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
            header: &'a [u8],
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
            header: &'a [u8],
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
        let (_, a2s_rules) = A2SRules::parse(&answer)?;
        Ok(a2s_rules.list)
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
        println!("{:?}", query.a2s_info_old().unwrap());
    }

    #[test]
    fn a2s_player_test() {
        let query = ValveQuery::bind("0.0.0.0:27515".parse().unwrap()).unwrap();
        query.set_timeout(Some(Duration::new(10, 0))).unwrap();
        query.connect(ADDR.parse().unwrap()).unwrap();
        let challenge = query.a2s_player_challenge().unwrap();
        let answer = query.a2s_players(challenge).unwrap();
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
