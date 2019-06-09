#![feature(log_syntax)]
//#![feature(async_await, await_macro, futures_api)]
//#![feature(trace_macros)]

//extern crate hyper;
//extern crate hyper_tls;

mod stm;

mod cal_machine;

fn main() {
    hyper::rt::run(cal_machine::run());
}
