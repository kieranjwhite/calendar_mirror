use std::io;
use std::net::{Ipv4Addr, TcpStream};

pub enum Operations {
    AddText(String, Pos, Id),
    UpdateText(Id, String),
    RemoveText(Id),
    Clear(),
    WriteAll(),
}

pub struct Pos(u32, u32);
pub struct Id(Option<String>);
pub struct Degrees(u32);

pub enum Timing {
    Immediate,
    Batched,
}

pub struct RenderPipeline {
    stream: TcpStream,
}

#[derive(Debug)]
pub enum Error {
    Network(io::Error),
}

impl From<io::Error> for Error {
    fn from(orig: io::Error) -> Error {
        Error::Network(orig)
    }
}

impl RenderPipeline {
    pub fn new() -> Result<RenderPipeline, Error> {
        Ok(RenderPipeline {
            stream: TcpStream::connect(("127.0.0.1", 443))?,
        })
    }
}
