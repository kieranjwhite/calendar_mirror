use crate::err;
use serde::Serialize;
use serde_json::error::Error as SerdeError;
use std::io::{self, BufWriter, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::slice::Iter;
use std::time::Duration;
use std::thread;

const DRIVER_PORT: u16 = 6029;
const SERVER_ADDR: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

#[allow(dead_code)]
#[derive(Serialize)]
pub enum Operation {
    AddText(String, Pos, Size, Id),
    UpdateText(Id, String),
    RemoveText(Id),
    Clear,
    WriteAll(PartialUpdate),
}

type Id = String;
type Size = u32;

#[derive(Serialize)]
pub struct PartialUpdate(pub bool);

#[derive(Serialize)]
pub struct Pos(pub u32, pub u32);

err!(Error {
    Network(io::Error),
    Serde(SerdeError)
});

pub struct RenderPipeline {
    stream: BufWriter<TcpStream>,
}

impl RenderPipeline {
    pub fn new() -> Result<RenderPipeline, Error> {
        let addr = SocketAddr::new(IpAddr::V4(SERVER_ADDR), DRIVER_PORT);
        Ok(RenderPipeline {
            stream: BufWriter::new(TcpStream::connect(addr)?),
        })
    }

    pub fn wait_for_server() -> Result<(), Error> {
        const CONNECT_INTERVAL: Duration=Duration::from_secs(1);
        
        let addr = SocketAddr::new(IpAddr::V4(SERVER_ADDR), DRIVER_PORT);
        let mut retries = 5;
        thread::sleep(CONNECT_INTERVAL);
        println!("attempt. retries remaining: {}", retries);
        let mut connection = TcpStream::connect(&addr);
        loop {
            if let Err(error) = connection {
                if retries == 0 {
                    return Err(error.into());
                }
                retries -= 1;
            } else {
                break;
            }
            thread::sleep(CONNECT_INTERVAL);
            println!("another attempt. retries remaining: {}", retries);
            connection = TcpStream::connect(&addr);
        }
        Ok(())
    }

    pub fn send(&mut self, els: Iter<Operation>) -> Result<(), Error> {
        for el in els {
            let serialised = serde_json::to_string(el)?;
            println!("sending: {}", serialised);
            write!(self.stream, "{}\n", serialised)?;
        }
        self.stream.flush()?;
        Ok(())
    }
}
