
/*
    start!(Load);
    tr!([Load, Wipe], RequestCodes);
    tr!([Load, Wait], Refresh);
    tr!([Refresh, Save], ReadFirst);
    tr!([RequestCodes, AuthPending, SlowPolling], Poll);
    tr!([RequestCodes, Poll], DisplayError);
    tr!([Poll], AuthPending);
    tr!([Poll], SlowPolling);
    tr!([Poll], Save);
    tr!([ReadFirst], Page);
    tr!([Page], Display);
    tr!([DisplayError, Display], Wait);
    tr!([Wait], Wipe);
    
    enum Machine {
        Load(Load),
    }
     */

trace_macros!(false);
stm!(create cal_stm, Load, {
    [Load ], RequestCodes;   
    [RequestCodes], ReadFirst
});

trace_macros!(false);
/*
pub mod stm {
    std!(Load, {
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
}
*/
pub fn run() {
    use cal_stm::*;

    let mut mach=Machine::new();
    loop {
        mach=match mach {
            Machine::Load(st) => {
                Machine::RequestCodes(st.into())
            }
            Machine::RequestCodes(st) => {
                Machine::ReadFirst(st.into())
            }
            Machine::ReadFirst(_st) => {
                break;
            }
        }
    }
}

