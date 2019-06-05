#![feature(trace_macros)]

#[macro_use]
mod std;

mod cal_machine;
mod retriever;
/*
macro_rules! loo {
    ($($($t: tt),+))+ => {
        $($(println!("{0}", $t);)*)*
    }
}
*/
fn main() {
    //loo!(1, 1);

    cal_machine::run();
}

