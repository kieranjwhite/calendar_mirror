//#![feature(trace_macros)]
//#![feature(log_syntax)]
mod cal_machine;
mod cal_display;
mod display;
mod err;
mod papirus_in;
mod stm;

use cal_machine::Error;

fn main() -> Result<(), Error> {
    cal_machine::run()
}
