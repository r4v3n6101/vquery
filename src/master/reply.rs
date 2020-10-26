use nom_derive::Nom;
use std::net::{Ipv4Addr, SocketAddrV4};

fn take_socket_addr(i: &[u8]) -> nom::IResult<&[u8], SocketAddrV4> {
    use nom::number::complete::{be_u16, le_u8};
    let (i, (first, second, third, fourth, port)) =
        nom::sequence::tuple((le_u8, le_u8, le_u8, le_u8, be_u16))(i)?;
    Ok((
        i,
        SocketAddrV4::new(Ipv4Addr::new(first, second, third, fourth), port),
    ))
}

#[derive(Nom)]
pub struct Reply<'a> {
    #[nom(Tag(b"\xFF\xFF\xFF\xFF\x66\x0A"))]
    _header: &'a [u8],
    #[nom(Parse = "nom::multi::many0(take_socket_addr)")]
    pub addresses: Vec<SocketAddrV4>,
}
