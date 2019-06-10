#![feature(log_syntax)]
mod cal_machine;
mod stm;

use reqwest::Result;

fn main() -> Result<()> {
    cal_machine::run()
}
