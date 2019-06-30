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
    env::{self, var_os},
    ffi::CString,
    fs::{self, create_dir_all},
    io,
    os::unix::fs::symlink,
    path::Path,
    process,
    sync::atomic::{AtomicBool, Ordering as AtomicOrdering},
    sync::Arc,
};

err!(
    Error {
        CalMachineError(CalMachineError),
        DisplayError(DisplayError),
        NixError(NixError),
        IOError(io::Error)
    }
);

#[derive(PartialEq, Eq)]
enum PackageAction {
    Install,
    Uninstall,
}

fn installation(
    action: PackageAction,
    package_install_dir: &Path,
    version: &str,
) -> Result<(), io::Error> {
    let script_rel_path: &Path = &Path::new(SCRIPTS_DIR).join(Path::new(SCRIPT_NAME));
    let exe_link: &Path = Path::new("/proc/self/exe");
    let bin_dir: &Path = Path::new("bin");

    let bin_path = package_install_dir.join(bin_dir);
    let exe_path = fs::read_link(exe_link)?;
    let exe_name = if let Some(exec_name) = exe_path.file_name() {
        exec_name
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "failed to identify filename of running executable",
        ));
    };

    let runnable_exe_path = bin_path.join(exe_name);
    let mut project_dir = exe_path.clone();

    for _ in 0..3 {
        project_dir = if let Some(dir) = project_dir.parent() {
            dir.to_path_buf()
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "failed to identify project directory",
            ));
        }
    }

    let script_path = project_dir.join(script_rel_path);
    let script_name = if let Some(script_name) = script_path.file_name() {
        script_name
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "failed to identify filename of script",
        ));
    };

    let runnable_script_path = bin_path.join(script_name);
    let version_path = package_install_dir.join(version);
    let version_exe = version_path.join(exe_name);
    let version_script = version_path.join(script_name);

    println!("begin uninstall");
    //always begin with an uninstall
    if version_exe.exists() {
        fs::remove_file(&version_exe)?;
    }
    println!("exe is gone {:?}", version_exe);
    if version_script.exists() {
        fs::remove_file(&version_script)?;
    }
    println!("script is gone {:?}", version_script);
    if version_path.exists() {
        fs::remove_dir(&version_path)?;
    }
    println!("version_path is gone {:?}", version_path);

    if package_install_dir.exists() {
        let num_dirs = package_install_dir.read_dir()?.count();
        if num_dirs == 1 {
            if bin_path.exists() {
                if let Ok(_)=fs::read_link(&runnable_exe_path) {
                    fs::remove_file(&runnable_exe_path)?;
                }
                if let Ok(_)=fs::read_link(&runnable_script_path) {
                    fs::remove_file(&runnable_script_path)?;
                }
                
                fs::remove_dir(&bin_path)?;
            }
            
            fs::remove_dir(&package_install_dir)?;
        }
    }
    println!("end uninstall");

    if action == PackageAction::Install {
        println!("begin install. version path: {:?} bin_path: {:?}", version_path, bin_path);
        println!("copying exe: from {:?} to {:?}. script: from {:?} to {:?} ", exe_path, version_exe, script_path, version_script);
        println!("links: exe {:?} script {:?}", runnable_exe_path, runnable_script_path);
        
        create_dir_all(&version_path)?;
        create_dir_all(&bin_path)?;

        if exe_path != version_exe {
            fs::copy(&exe_path, &version_exe)?;
        }

        if script_path != version_script {
            fs::copy(&script_path, &version_script)?;
        }

        symlink(&version_exe, &runnable_exe_path)?;
        symlink(&version_script, &runnable_script_path)?;
        println!("end install");
    }

    Ok(())
}

const SCRIPTS_DIR: &str = "scripts";
const SCRIPT_NAME: &str = "calendar_mirror_server.py";
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_VAR_DIR: &str=".";

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let dest_base: &Path = Path::new("/opt/");
    for arg in args.iter() {
        match arg.as_str() {
            "--install" => {
                installation(PackageAction::Install, &dest_base.join(PKG_NAME), VERSION)?;
                process::exit(0);
            }
            "--uninstall" => {
                installation(PackageAction::Uninstall, &dest_base.join(PKG_NAME), VERSION)?;
                process::exit(0);
            }
            _ => {}
        }
    }

    let var_dir_opt=var_os("CALENDAR_MIRROR_VAR");
    let var_dir=if let Some(ref val)=var_dir_opt {
        Path::new(val)
    } else {
        Path::new(DEFAULT_VAR_DIR)
    };

    //const PYTHON_NAME: &str = "/usr/bin/python3";
    let quitter = Arc::new(AtomicBool::new(false));
    if cfg!(feature = "render_stm") {
        let mut renderer = Renderer::wait_for_server()?;
        cal_machine::run(&mut renderer, quitter, var_dir)?;
    } else {
        match fork().expect("fork failed") {
            ForkResult::Parent { child: _ } => {
                let child_quitter = Arc::clone(&quitter);
                println!("parent is waiting for child to start server...");
                let mut renderer = Renderer::wait_for_server()?;
                ctrlc::set_handler(move || {
                    child_quitter.store(true, AtomicOrdering::SeqCst);
                })
                .expect("Error setting Ctrl-C handler");
                renderer.disconnect_quits_server()?;
                cal_machine::run(&mut renderer, quitter, var_dir)?;

                println!("finishing up");
            }
            ForkResult::Child => {
                println!("child will now start server...");
                execvp(
                    &CString::new(SCRIPT_NAME)
                        .expect(&format!("Invalid CString: {}", SCRIPT_NAME)),
                    &[]
                )?;
            }
        }
    }
    Ok(())
}
