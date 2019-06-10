mod retriever;

use crate::stm;
use reqwest::{Response, Result};
use retriever::{DeviceUserCodeResponse, EventRetriever};

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

pub fn run() -> Result<()> {
    use cal_stm::Machine;
    use cal_stm::Machine::*;

    let retriever = EventRetriever::inst();
    let mut mach: Machine = Machine::new_stm();
    loop {
        mach = match mach {
            Load(st) => RequestCodes(st.into()),
            RequestCodes(st) => {
                let mut resp: Response = retriever.retrieve_dev_and_code()?;
                println!("Headers: {:#?}", resp.headers());

                let body: DeviceUserCodeResponse = resp.json()?;

                println!("Body is next... {:?}", body);
                Poll(st.into())
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
