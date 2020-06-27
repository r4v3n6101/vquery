use either::Either;
use nom_derive::*;
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
        let (_, a2s_challenge) = A2SChallenge::parse(&answer).unwrap(); // TODO :
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
        let (_, a2s_info_old) = A2SInfoOld::parse(&answer).unwrap(); // TODO
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
        let (_, a2s_info_new) = A2SInfoNew::parse(&answer).unwrap(); // TODO
        Ok(a2s_info_new.info)
    }

    pub fn a2s_info(&self) -> QueryResult<Either<InfoNew, InfoOld>> {
        unimplemented!()
    }

    /*pub fn a2s_player(&self, challenge: i32) -> QueryResult<Vec<A2SPlayer>> {
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
    } */
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

    /*#[test]
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
    }*/
}
