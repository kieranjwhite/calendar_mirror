use crate::err;
use serde::Serialize;
use serde_json::error::Error as SerdeError;
use std::io::{self, BufWriter, Write};
use std::net::TcpStream;
use std::slice::Iter;

pub const DRIVER_PORT: u16 = 6029;

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
        Ok(RenderPipeline {
            stream: BufWriter::new(TcpStream::connect(("127.0.0.1", DRIVER_PORT))?),
        })
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
