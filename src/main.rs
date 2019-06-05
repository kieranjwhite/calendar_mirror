#[macro_use]
mod stm;

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

