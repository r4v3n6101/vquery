use futures::{StreamExt, TryStreamExt};
use std::{io, sync::Arc};
use tokio::net::UdpSocket;
use vquery::master::{Region, ServerQuery};

const MASTER_ADDR: &str = "hl2master.steampowered.com:27011";

#[tokio::test]
async fn test_take_500_addresses() -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(MASTER_ADDR).await?;

    let socket = Arc::new(socket);
    let server_query = ServerQuery::new(Region::Europe, &[]);

    server_query
        .addresses(socket)
        .into_stream()
        .take(500)
        .for_each(|addr| async move { println!("{}", addr.unwrap().to_string()) })
        .await;
    Ok(())
}
