mod retriever;

use crate::stm;
use hyper::Chunk;
use hyper::client::ResponseFuture;
use hyper::rt::{Future, Stream};
use hyper::StatusCode;
use retriever::{DeviceUserCodeResponse,EventRetriever};
use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::io;
use std::io::Write;

stm!(cal_stm, Load, {
    [Load, Wipe], RequestCodes;
    [Load, Wait], Refresh;
    [Refresh, Save], ReadFirst;
    [RequestCodes, AuthPending, SlowPolling], Poll;
    [RequestCodes, Poll], DisplayError;
    [Poll], AuthPending;
    [Poll], SlowPolling;
    [Poll], Save;
    [ReadFirst], Page;
    [Page], Display;
    [DisplayError, Display], Wait;
    [Wait], Wipe
});

pub fn run() -> impl Future<Item = (), Error = ()> {
    use cal_stm::Machine;
    use cal_stm::Machine::*;

    let retriever = EventRetriever::inst();
    let mut resp: ResponseFuture;
    let mut mach: Machine = Machine::new_stm();
    let mut final_future;
    loop {
        mach = match mach {
            Load(st) => RequestCodes(st.into()),
            RequestCodes(st) => {
                resp = retriever.retrieve_dev_and_code();
                final_future = resp
                    .map(|resp| {
                        /*
                        match resp.status() {
                            StatusCode::OK => {
                                
                            }
                        }
                        */
                        println!("POST: {}", resp.status());
                        println!("Headers: {:#?}", resp.headers());

                        let response=resp.body().concat2().map_err(|_err| ()).map(|chunk| {
                            let v = chunk.to_vec();
                            String::from_utf8_lossy(&v).to_string()
                        });
                        let response: Result<DeviceUserCodeResponse> = serde_json::from_str(response);
                        
                        println!("Body is next...");
                        // The body is a stream, and for_each returns a new Future
                        // when the stream is finished, and calls the closure on
                        // each chunk of the body...
                        //resp.into_body().for_each(|chunk| {
                        //    io::stdout()
                        //        .write_all(&chunk)
                        //        .map_err(|e| panic!("example expects stdout is open, error={}", e))
                        //})
                        println!("\n\nDone.");
                    })
                    .map_err(|err| {
                        eprintln!("Error {}", err);
                    });
                return final_future;
                //RequestCodes(st)
            }
            Refresh(st) => Refresh(st),
            ReadFirst(st) => ReadFirst(st),
            Poll(st) => Poll(st),
            DisplayError(st) => DisplayError(st),
            AuthPending(st) => AuthPending(st.into()),
            SlowPolling(st) => SlowPolling(st),
            Save(st) => Save(st),
            Page(st) => Page(st),
            Display(st) => Display(st),
            Wait(st) => Wait(st),
            Wipe(st) => Wipe(st),
        };
    }
}
