use ipstack::stream::IpStackStream;
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use tun2::{self, Configuration};

//use tproxy_config;
use std::sync::mpsc::channel;

async fn my_bidirection_copy<L, R>(lhs: L, rhs: R, addr: String)
where
    L: AsyncRead + AsyncWrite + Send + Sync + 'static,
    R: AsyncRead + AsyncWrite + Send + Sync + 'static,
{
    let (mut l_reader, mut l_writer) = tokio::io::split(lhs);
    let (mut r_reader, mut r_writer) = tokio::io::split(rhs);
    let mut join_set = tokio::task::JoinSet::new();
    let _addr1 = addr.clone();
    join_set.spawn(async move {
        let mut buf = [0u8; 1500];
        loop {
            let size = match tokio::time::timeout(
                std::time::Duration::from_secs(3),
                l_reader.read(&mut buf),
            )
            .await
            {
                Ok(v) => v?,
                Err(_e) => {
                    println!("read from ipstack timeout {_e:?}");
                    // r_writer.shutdown().await.unwrap();
                    // println!("left close ok for {addr1}");
                    return anyhow::Ok(());
                }
            };
            if size == 0 {
                //println!("left 0 {addr1}");
                //r_writer.shutdown().await.unwrap();
                //println!("left close ok for {addr1}");
                return anyhow::Ok(());
            }
            //println!("outbound {}",String::from_utf8_lossy(&buf[..size]));
            r_writer.write_all(&buf[..size]).await?;
        }
    });
    let _addr2 = addr.clone();
    join_set.spawn(async move {
        let mut buf = [0u8; 1500];
        loop {
            let size = match tokio::time::timeout(
                std::time::Duration::from_secs(3),
                r_reader.read(&mut buf),
            )
            .await
            {
                Ok(v) => v?,
                Err(_e) => {
                    println!("read from server timeout {_e:?}");
                    //l_writer.shutdown().await.unwrap();
                    return anyhow::Ok(());
                }
            };
            if size == 0 {
                //println!("right read 0 {addr2}");
                //l_writer.shutdown().await.unwrap();
                //println!("right close ok for {addr2}");
                return anyhow::Ok(());
            }
            //println!("inbound {}", String::from_utf8_lossy(&buf[..size]));
            l_writer.write_all(&buf[..size]).await?;
        }
    });
    while let Some(_) = join_set.join_next().await {
        //break;
    }
    println!("====== end tcp connection ====== {addr}");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let (tx, rx) = channel();
    let _ = ctrlc2::set_handler(move || {
        tx.send(()).expect("Could not send signal on channel.");
        true
    })
    .expect("Error setting Ctrl-C handler");
    let mut config = Configuration::default();
    config
        .address((10, 0, 0, 9))
        .netmask((255, 255, 255, 0))
        //.destination((10, 0, 0, 1))
        .tun_name("utun6")
        .up();
    let mut ipstack_config = ipstack::IpStackConfig::default();
    ipstack_config.mtu(u16::MAX);

    let mut ip_stack =
        ipstack::IpStack::new(ipstack_config, tun2::create_as_async(&config).unwrap());

    println!("starting!!!!");

    tokio::spawn(async move {
        while let Ok(stream) = ip_stack.accept().await {
            match stream {
                IpStackStream::Tcp(tcp) => {
                    println!("IpStackStream::Tcp(tcp) {} -> {}", tcp.local_addr(), tcp.peer_addr());
                    let addr = tcp.peer_addr().ip().to_string();

                    let s = match tokio::net::TcpStream::connect(SocketAddr::V4("101.35.230.139:8080".parse().unwrap())).await {
                        Ok(s) => s,
                        Err(e) => {
                            println!("connect TCP server failed \"{}\"", e);
                            continue;
                        }
                    };
                    tokio::spawn(my_bidirection_copy(tcp, s,addr));
                    // tokio::spawn(async move {
                    //     let _ = tokio::io::copy_bidirectional(&mut tcp, &mut s).await;
                    //     println!("====== end tcp connection ======");
                    // });
                }
                IpStackStream::Udp(_udp) => {}
                IpStackStream::UnknownNetwork(_) => {}
                IpStackStream::UnknownTransport(_) => {}
            }
        }
        anyhow::Ok(())
    });

    rx.recv().expect("Could not receive from channel.");

    println!("terminate the program");
    Ok(())
}


