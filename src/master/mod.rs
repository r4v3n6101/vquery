use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    io::Result as IOResult,
    net::{SocketAddr, SocketAddrV4, UdpSocket},
    time::Duration,
};

mod reply;
use reply::Reply;

const BUF_SIZE: usize = 2 << 20; // 1Mb

pub enum Region {
    UsEastCost = 0x00,
    UsWestCost = 0x01,
    SouthAmerica = 0x02,
    Europe = 0x03,
    Asia = 0x04,
    Australia = 0x05,
    MiddleEast = 0x06,
    Africa = 0x07,
    All = 0xFF,
}

pub enum Filter<'a> {
    Nor(Vec<Self>),
    Nand(Vec<Self>),
    Dedicated,
    Secure,
    GameDir(&'a str),
    Map(&'a str),
    Linux,
    NoPassword,
    NotEmpty,
    NotFull,
    Proxy,
    Appid(&'a str),
    NotAppid(&'a str),
    NoPlayers,
    Whitelisted,
    //GameType(..),
    //GameData(..),
    //GameDataOr(..), TODO
    NameMatch(&'a str),
    VersionMatch(&'a str),
    CollapseAddrHash,
    GameAddr(SocketAddr),
}

impl Display for Filter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Filter::Nor(vec) => write!(f, "\\nor\\{}", vec.len())
                .and(vec.iter().try_for_each(|filter| write!(f, "{}", filter))),
            Filter::Nand(vec) => write!(f, "\\nand\\{}", vec.len())
                .and(vec.iter().try_for_each(|filter| write!(f, "{}", filter))),
            Filter::Dedicated => write!(f, "\\dedicated\\1"),
            Filter::Secure => write!(f, "\\secure\\1"),
            Filter::GameDir(dir) => write!(f, "\\gamedir\\{}", dir),
            Filter::Map(map) => write!(f, "\\map\\{}", map),
            Filter::Linux => write!(f, "\\linux\\1"),
            Filter::NoPassword => write!(f, "\\password\\0"),
            Filter::NotEmpty => write!(f, "\\empty\\1"),
            Filter::NotFull => write!(f, "\\full\\1"),
            Filter::Proxy => write!(f, "\\proxy\\1"),
            Filter::Appid(appid) => write!(f, "\\appid\\{}", appid),
            Filter::NotAppid(appid) => write!(f, "\\napp\\{}", appid),
            Filter::NoPlayers => write!(f, "\\noplayers\\1"),
            Filter::Whitelisted => write!(f, "\\white\\1"),
            // TODO
            Filter::NameMatch(hostname) => write!(f, "\\name_match\\{}", hostname),
            Filter::VersionMatch(version) => write!(f, "\\version_match\\{}", version),
            Filter::CollapseAddrHash => write!(f, "\\collaspse_addr_hash\\1"),
            Filter::GameAddr(addr) => write!(f, "\\gameaddr\\{}", addr),
        }
    }
}

pub struct MasterServerQuery(UdpSocket);

impl MasterServerQuery {
    pub fn bind(addr: SocketAddr) -> IOResult<Self> {
        UdpSocket::bind(addr).map(Self)
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

    fn raw_request<A: AsRef<[u8]>, F: AsRef<[u8]>>(
        &self,
        message_type: u8,
        region_code: u8,
        addr: A,
        filter: F,
    ) -> IOResult<Vec<u8>> {
        let addr = addr.as_ref();
        let filter = filter.as_ref();
        let mut data = Vec::with_capacity(4 + addr.len() + filter.len());
        data.push(message_type);
        data.push(region_code);
        data.extend(addr);
        data.push(0);
        data.extend(filter);
        data.push(0);

        self.0.send(&data)?;

        let mut buf = vec![0; BUF_SIZE]; // preallocation of 1mb is enough I think
        let size = self.0.recv(&mut buf)?;
        buf.truncate(size);
        Ok(buf)
    }

    pub fn request(&self, region: Region, filters: &[Filter]) -> Vec<SocketAddrV4> {
        let data = self
            .raw_request(
                0x31,
                region as u8,
                "0.0.0.0:0".to_string(), // TODO : recall many times in iterator to get all data
                filters.iter().map(|f| format!("{}", f)).collect::<String>(),
            )
            .unwrap();
        let (_, reply) = Reply::parse(&data).unwrap(); // TODO : remove unwrap
        reply.addresses
    }
}
