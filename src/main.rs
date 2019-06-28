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
use std::{
    ffi::CString,
    sync::atomic::{AtomicBool, Ordering as AtomicOrdering},
    sync::Arc,
};
err!(
    Error {
        CalMachineError(CalMachineError),
        DisplayError(DisplayError),
        NixError(NixError)
    }
);

fn main() -> Result<(), Error> {
    //const PYTHON_NAME: &str = "/usr/bin/python3";
    const SCRIPT_PATH: &str = "scripts/server.py";
    let quitter = Arc::new(AtomicBool::new(false));
    if cfg!(feature = "render_stm") {
        let mut renderer = Renderer::wait_for_server()?;
        cal_machine::run(&mut renderer, quitter)?;
    } else {
        match fork().expect("fork failed") {
            ForkResult::Parent { child: _ } => {
                let child_quitter = Arc::clone(&quitter);
                ctrlc::set_handler(move || {
                    child_quitter.store(true, AtomicOrdering::SeqCst);
                })
                .expect("Error setting Ctrl-C handler");
                {
                    println!("parent is waiting for child to start server...");
                    let mut renderer = Renderer::wait_for_server()?;
                    renderer.disconnect_quits_server()?;
                    cal_machine::run(&mut renderer, quitter)?;
                }
                println!("finishing up");
            }
            ForkResult::Child => {
                println!("child will now start server...");
                execv(
                    &CString::new(SCRIPT_PATH).expect(&format!("Invalid CString: {}", SCRIPT_PATH)),
                    &[],
                )?;
            }
        }
    }
    Ok(())
}
