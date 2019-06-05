
stm!(create cal_stm, Load, {
    [Load ], RequestCodes;   
    [RequestCodes], ReadFirst
});
/*
    stm!(create cal_stm, Load, {
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
*/
pub fn run() {
    use cal_stm::*;

    let mut mach=Machine::new();
    let mut cnt=0;
    loop {
        mach=match mach {
            Machine::Load(st) => {
                println!("next state");
                Machine::RequestCodes(st.into())
            }
            Machine::RequestCodes(st) => {
                if cnt>=10
                {
                    println!("now at 10");
                    Machine::ReadFirst(st.into())
                }
                else {
                    cnt+=1;
                    println!("increment");
                    Machine::RequestCodes(st)
                }
            }
            Machine::ReadFirst(_st) => {
                println!("at end");
                break;
            }
        }
    }
}

