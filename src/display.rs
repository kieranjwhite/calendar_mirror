use crate::err;
use serde::Serialize;
use serde_json::error::Error as SerdeError;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::ops::Add;
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
    Sync,
    QuitWhenDone,
}

type Id = String;
type Size = u32;

#[derive(Serialize)]
pub struct PartialUpdate(pub bool);

#[derive(Serialize)]
pub struct VertPos(pub i32);

#[derive(Serialize)]
pub struct Pos(pub i32, pub u32);

impl Add<&VertPos> for &Pos {
    type Output=Pos;

    fn add(self, rhs: &VertPos) -> Pos {
        Pos(self.0+rhs.0, self.1)
    }
}

err!(Error {
    Network(io::Error),
    Serde(SerdeError)
});

pub struct RenderPipeline {
    r_stream: BufReader<TcpStream>,
    w_stream: BufWriter<TcpStream>,
}

impl RenderPipeline {
    pub fn new() -> Result<RenderPipeline, Error> {
        let addr = SocketAddr::new(IpAddr::V4(SERVER_ADDR), DRIVER_PORT);
        let r_connection=TcpStream::connect(addr)?;
        let w_connection=r_connection.try_clone()?;
        Ok(RenderPipeline {
            r_stream: BufReader::new(r_connection),
            w_stream: BufWriter::new(w_connection),
        })
    }

    pub fn wait_for_server() -> Result<(), Error> {
        const CONNECT_INTERVAL: Duration=Duration::from_secs(1);
        
        let addr = SocketAddr::new(IpAddr::V4(SERVER_ADDR), DRIVER_PORT);
        let mut retries = 20;
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

    pub fn send(&mut self, els: Iter<Operation>, sync: bool) -> Result<(), Error> {
        for el in els {
            let serialised = serde_json::to_string(el)?;
            println!("sending: {}", serialised);
            write!(self.w_stream, "{}\n", serialised)?;
        }
        if sync {
            let serialised = serde_json::to_string(&Operation::Sync)?;
            write!(self.w_stream, "{}\n", serialised)?;
            self.w_stream.flush()?;

            let mut line=String::new();
            self.r_stream.read_line(&mut line)?;
        } else {
            self.w_stream.flush()?;
        }
        Ok(())
    }
}

