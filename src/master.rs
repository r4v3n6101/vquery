use combine::{
    any, many,
    parser::{byte::num::be_u16, range::range},
    Parser,
};
use futures::stream::{self, TryStream, TryStreamExt};
use std::{
    fmt,
    io::{self, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tokio::net::UdpSocket;

const BUF_SIZE: usize = 4 * 1024; // 4kb should be fine
const MESSAGE_TYPE: u8 = 0x31;
const REPLY_HEADER: [u8; 6] = [0xFF, 0xFF, 0xFF, 0xFF, 0x66, 0x0A];

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

#[derive(Clone)]
pub enum Filter {
    Nor(Vec<Self>),
    Nand(Vec<Self>),
    Dedicated,
    Secure,
    GameDir(String),
    Map(String),
    Linux,
    NoPassword,
    NotEmpty,
    NotFull,
    Proxy,
    Appid(String),
    NotAppid(String),
    NoPlayers,
    Whitelisted,
    GameType(String),
    GameDataAll(String),
    GameDataAny(String),
    NameMatch(String),
    VersionMatch(String),
    CollapseAddrHash,
    GameAddr(SocketAddr),
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            Filter::GameType(gtype) => write!(f, "\\gametype\\{}", gtype),
            Filter::GameDataAll(gdata) => write!(f, "\\gamedata\\{}", gdata),
            Filter::GameDataAny(gdata) => write!(f, "\\gamedataor\\{}", gdata),
            Filter::NameMatch(hostname) => write!(f, "\\name_match\\{}", hostname),
            Filter::VersionMatch(version) => write!(f, "\\version_match\\{}", version),
            Filter::CollapseAddrHash => write!(f, "\\collaspse_addr_hash\\1"),
            Filter::GameAddr(addr) => write!(f, "\\gameaddr\\{}", addr),
        }
    }
}

pub struct ServerQuery {
    region_code: u8,
    filter: Vec<u8>,
}

impl ServerQuery {
    pub fn new<A: AsRef<[Filter]>>(region: Region, filters: A) -> ServerQuery {
        let mut filter_buf = Vec::new();
        filters
            .as_ref()
            .iter()
            .try_for_each(|filter| write!(&mut filter_buf, "{}", filter))
            .unwrap(); // unwrap as <T as Display>::to_string does
        Self {
            region_code: region as u8,
            filter: filter_buf,
        }
    }

    #[inline]
    fn packet_data(region_code: u8, filter: &[u8], seed: &[u8]) -> Vec<u8> {
        let mut data = Vec::with_capacity(4 + filter.len() + seed.len());
        data.push(MESSAGE_TYPE);
        data.push(region_code);
        data.extend(seed);
        data.push(0);
        data.extend(filter);
        data.push(0);
        data
    }

    #[inline]
    fn parse_reply(data: &[u8]) -> io::Result<Vec<SocketAddrV4>> {
        let socket = (any(), any(), any(), any(), be_u16())
            .map(|(a, b, c, d, port)| SocketAddrV4::new(Ipv4Addr::new(a, b, c, d), port));
        range(REPLY_HEADER.as_ref())
            .with(many(socket))
            .parse(data)
            .map(|(data, _)| data)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "couldn't parse response"))
    }

    async fn raw_request(socket: &UdpSocket, data: &[u8]) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; BUF_SIZE];

        socket.send(data).await?;
        let size = socket.recv(&mut buf).await?;
        buf.truncate(size);

        Ok(buf)
    }

    async fn request(
        &self,
        socket: &UdpSocket,
        seed: &SocketAddrV4,
    ) -> io::Result<Vec<SocketAddrV4>> {
        let packet = Self::packet_data(self.region_code, &self.filter, seed.to_string().as_bytes());
        let response = Self::raw_request(socket, &packet).await?;
        let reply = Self::parse_reply(&response)?;
        Ok(reply)
    }

    pub fn addresses(
        &self,
        socket: Arc<UdpSocket>,
    ) -> impl TryStream<Ok = SocketAddrV4, Error = io::Error> + '_ {
        let nul_adress = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0);
        stream::try_unfold(Some(nul_adress), move |state| {
            let socket = socket.clone();
            async move {
                if let Some(seed) = state {
                    let reply = self.request(&socket, &seed).await?;
                    let next_state = reply
                        .last()
                        .copied()
                        .filter(|next_seed| next_seed != &nul_adress);
                    Ok(Some((
                        stream::iter(
                            reply
                                .into_iter()
                                .filter(move |addr| addr != &nul_adress)
                                .map(Ok),
                        ),
                        next_state,
                    ))) as io::Result<_>
                } else {
                    Ok(None)
                }
            }
        })
        .try_flatten()
    }
}
