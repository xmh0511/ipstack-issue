use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main(){
	let mut client = tokio::net::TcpStream::connect("10.0.0.6:6").await.unwrap();
	let mut buf = [0u8;15000];
	client.write_all(b"GET / HTTP/1.1\r\n\r\n").await.unwrap();
	loop {
		tokio::select! {
			v = client.read(& mut buf) =>{
				match v{
					Ok(size)=>{
						println!("read size  = {}",size);
						if size == 0{
							println!("end!!!!");
							return;
						}
					}
					Err(e)=>{
						println!("{e:?}");
						return;
					}
				}
			}
			_ = tokio::time::sleep(std::time::Duration::from_secs(6))=>{
				println!("continue");
				client.write_all(b"GET / HTTP/1.1\r\n\r\n").await.unwrap();
			}
		}
	}
}