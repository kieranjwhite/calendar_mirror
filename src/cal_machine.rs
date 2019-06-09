mod retriever;

use crate::stm;
use hyper::client::ResponseFuture;
use hyper::rt::{Future, Stream};
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

    let retriever = retriever::EventRetriever::inst();
    let mut resp: ResponseFuture;
    let mut mach: Machine = Machine::new_stm();
    let mut final_future;
    loop {
        mach = match mach {
            Load(st) => RequestCodes(st.into()),
            RequestCodes(st) => {
                resp = retriever.retrieve_dev_and_code();
                final_future = resp
                    .and_then(|res| {
                        println!("POST: {}", res.status());
                        println!("Headers: {:#?}", res.headers());

                        // The body is a stream, and for_each returns a new Future
                        // when the stream is finished, and calls the closure on
                        // each chunk of the body...
                        res.into_body().for_each(|chunk| {
                            io::stdout()
                                .write_all(&chunk)
                                .map_err(|e| panic!("example expects stdout is open, error={}", e))
                        })
                    })
                    .map(|_| {
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
