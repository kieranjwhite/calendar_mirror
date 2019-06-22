//#![feature(trace_macros)]
//#![feature(log_syntax)]
mod cal_machine;
mod display;
mod stm;

use cal_machine::Error;

fn main() -> Result<(), Error> {
    cal_machine::run()
}
