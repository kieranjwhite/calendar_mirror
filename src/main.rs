//#![feature(trace_macros)]
//#![feature(log_syntax)]
mod cal_display;
mod cal_machine;
mod display;
mod err;
mod papirus_in;
mod stm;

use cal_display::Renderer;
use cal_machine::Error as CalMachineError;
use display::Error as DisplayError;
use nix::{unistd::*, Error as NixError};
use std::ffi::CString;

err!(
    Error {
        CalMachineError(CalMachineError),
        DisplayError(DisplayError),
        NixError(NixError)
    }
);

fn main() -> Result<(), Error> {
    const PYTHON_NAME: &str = "/usr/bin/python3";
    const SCRIPT_PATH: &str = "scripts/server.py";
    if cfg!(feature = "render_stm") {
        cal_machine::run()?;
    } else {
        match fork().expect("fork failed") {
            ForkResult::Parent { child: _ } => {
                println!("parent is waiting for child to start server...");
                Renderer::wait_for_server()?;
                cal_machine::run()?;
            }
            ForkResult::Child => {
                println!("child will now start server...");
                execv(
                    &CString::new(SCRIPT_PATH).expect(&format!("Invalid CString: {}", SCRIPT_PATH)),
                    &[],
                )?;
                //execv(
                //   &CString::new(PYTHON_NAME).expect(&format!("Invalid CString: {}", PYTHON_NAME)),
                //   [&CString::new(SCRIPT_PATH).expect(&format!("Invalid CString: {}", SCRIPT_PATH))],
                //)?;
            }
        }
    }
    Ok(())
}
