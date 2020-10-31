use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    io::Result as IOResult,
    iter::Iterator,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
    time::Duration,
};

mod reply;
use reply::Reply;
mod error;
use error::QueryResult;

const BUF_SIZE: usize = 2 << 20; // 1Mb
#[derive(Copy, Clone)]
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

pub struct ServersQuery(UdpSocket);

impl ServersQuery {
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

    pub fn request(
        &self,
        seed: &SocketAddrV4,
        region: Region,
        filters: &[Filter],
    ) -> QueryResult<Vec<SocketAddrV4>> {
        let data = self.raw_request(
            0x31,
            region as u8,
            seed.to_string(),
            filters.iter().map(|f| format!("{}", f)).collect::<String>(),
        )?;

        let (_, reply) = Reply::parse(&data)?;
        Ok(reply.addresses)
    }

    pub fn iter<'a>(&'a self, region: Region, filters: &'a [Filter<'a>]) -> MasterQueryIter<'a> {
        MasterQueryIter::new(self, region, filters)
    }
}

pub struct MasterQueryIter<'a> {
    region: Region,
    filters: &'a [Filter<'a>],
    query: &'a ServersQuery,
    buf: Vec<SocketAddrV4>,
    index: usize,
}

impl<'a> MasterQueryIter<'a> {
    fn new(query: &'a ServersQuery, region: Region, filters: &'a [Filter<'a>]) -> Self {
        Self {
            region,
            filters,
            query,
            buf: vec![],
            index: 0,
        }
    }
}

impl<'a> Iterator for MasterQueryIter<'a> {
    type Item = QueryResult<SocketAddrV4>;

    fn next(&mut self) -> Option<Self::Item> {
        let nul_addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0);
        let mut val = self.buf.get(self.index);

        if val.is_none() {
            let seed = self.buf.last().unwrap_or(&nul_addr);
            if seed == &nul_addr {
                self.index = 0;
            } else {
                self.index = 1;
            }
            let reply = self.query.request(seed, self.region, self.filters);
            match reply {
                Ok(reply) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&reply);
                    val = self.buf.get(self.index);
                }
                Err(err) => return Some(Err(err)),
            }
        }
        self.index += 1;
        val.copied().map(Ok)
    }
}
