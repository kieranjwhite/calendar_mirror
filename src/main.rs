#[macro_use]
mod cal_display;
mod cal_machine;
mod display;
mod err;
pub mod formatter;
mod gpio_in;
#[macro_use]
mod new_type;
mod stm;
#[allow(bare_trait_objects)]
mod systemd1;
//mod yielder;

use cal_display::{Error as CalDisplayError, Renderer};
use cal_machine::{Error as CalMachineError, RefreshToken};
use dbus::{BusType, Connection};
use display::Error as DisplayError;
use nix::{mount::*, unistd::*, Error as NixError};
use std::{
    env::{self, var_os},
    ffi::CString,
    fs::{self, create_dir_all},
    io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::{self, Command},
    sync::atomic::{AtomicBool, Ordering as AtomicOrdering},
    sync::Arc,
    thread,
    time::Duration,
};
use systemd1::OrgFreedesktopSystemd1Manager;

fn system_d_inst<'a>(
    dbus: &'a Connection,
) -> Result<dbus::ConnPath<'a, &'a dbus::Connection>, Error> {
    let system_d = dbus.with_path(
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        5000,
    );

    Ok(system_d)
}

#[derive(Debug)]
pub struct PathError(PathBuf);

#[derive(Clone, Debug)]
pub struct UninitialisedStaticError();

err!(
    Error {
        CalMachineError(CalMachineError),
        DisplayError(DisplayError),
        CalDisplayError(CalDisplayError),
        NixError(NixError),
        IOError(io::Error),
        DBus(dbus::Error),
        Path(PathError),
        UninitialisedStatic(UninitialisedStaticError)
    }
);

#[derive(PartialEq, Eq)]
enum PackageAction {
    Install,
    Uninstall,
}

fn installation<'a>(
    system_d: &'a dbus::ConnPath<'a, &'a dbus::Connection>,
    action: PackageAction,
    install_dir: &Path,
    version: &str,
) -> Result<(), Error> {
    let package_install_dir = &install_dir.join(PKG_NAME);
    let script_rel_path: &Path = &Path::new(SCRIPTS_DIR).join(Path::new(SCRIPT_NAME));
    let systemd_rel_path: &Path =
        &Path::new(SYSTEMD_DIR).join(Path::new(CALENDAR_MIRROR_UNIT_NAME));
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
        )
        .into());
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
            )
            .into());
        }
    }

    let script_path = project_dir.join(script_rel_path);
    let script_name = Path::new(SCRIPT_NAME);
    let systemd_path = project_dir.join(systemd_rel_path);
    let unit_name = Path::new(CALENDAR_MIRROR_UNIT_NAME);

    let runnable_script_path = bin_path.join(script_name);
    let version_path = package_install_dir.join(version);
    let version_exe = version_path.join(exe_name);
    let version_script = version_path.join(script_name);
    let version_unit = version_path.join(unit_name);

    println!("begin uninstall");
    //always begin with an uninstall

    match system_d.stop_unit(CALENDAR_MIRROR_UNIT_NAME, UNIT_STOP_START_CONFIG) {
        Ok(_) => {
            println!("disabled the unit {:?}", CALENDAR_MIRROR_UNIT_NAME);
        }
        Err(err) => {
            println!("error disabling {:?}", err);
        }
    }

    println!("disabling the unit {:?}", CALENDAR_MIRROR_UNIT_NAME);
    match system_d.disable_unit_files(vec![CALENDAR_MIRROR_UNIT_NAME], false) {
        Ok(_) => {
            println!("disabled the unit {:?}", CALENDAR_MIRROR_UNIT_NAME);
        }
        Err(err) => {
            println!("error disabling {:?}", err);
        }
    }

    if version_exe.exists() {
        fs::remove_file(&version_exe)?;
    }
    println!("exe is gone {:?}", version_exe);
    if version_script.exists() {
        fs::remove_file(&version_script)?;
    }
    println!("script is gone {:?}", version_script);
    if version_unit.exists() {
        fs::remove_file(&version_unit)?;
    }
    println!("unit is gone {:?}", version_unit);
    if version_path.exists() {
        fs::remove_dir(&version_path)?;
    }
    println!("version_path is gone {:?}", version_path);

    if package_install_dir.exists() {
        if bin_path.exists() {
            if let Ok(_) = fs::read_link(&runnable_exe_path) {
                fs::remove_file(&runnable_exe_path)?;
            }
            if let Ok(_) = fs::read_link(&runnable_script_path) {
                fs::remove_file(&runnable_script_path)?;
            }

            fs::remove_dir(&bin_path)?;
        }

        let num_dirs = package_install_dir.read_dir()?.count();
        if num_dirs == 0 {
            fs::remove_dir(&package_install_dir)?;
        }
    }
    println!("end uninstall");

    if action == PackageAction::Install {
        println!(
            "begin install. version path: {:?} bin_path: {:?}",
            version_path, bin_path
        );
        println!(
            "copying exe: from {:?} to {:?}. script: from {:?} to {:?}, unit: from {:?} to {:?} ",
            exe_path, version_exe, script_path, version_script, systemd_path, version_unit
        );
        println!(
            "links: exe {:?} script {:?}",
            runnable_exe_path, runnable_script_path
        );

        create_dir_all(&version_path)?;
        create_dir_all(&bin_path)?;

        if exe_path != version_exe {
            fs::copy(&exe_path, &version_exe)?;
        }

        if script_path != version_script {
            fs::copy(&script_path, &version_script)?;
        }

        if systemd_path != version_unit {
            fs::copy(&systemd_path, &version_unit)?;
        }

        symlink(&version_exe, &runnable_exe_path)?;
        symlink(&version_script, &runnable_script_path)?;

        println!("linking the unit {:?}", version_unit);
        if let Some(version_unit_str) = version_unit.to_str() {
            system_d.link_unit_files(vec![version_unit_str], false, false)?;
        } else {
            return Err(PathError(version_unit.to_path_buf()).into());
        }
        println!("linked the unit {:?}", version_unit);

        println!("enabling the unit {:?}", CALENDAR_MIRROR_UNIT_NAME);
        system_d.enable_unit_files(vec![CALENDAR_MIRROR_UNIT_NAME], false, false)?;
        println!("starting the unit {:?}", CALENDAR_MIRROR_UNIT_NAME);
        system_d.start_unit(CALENDAR_MIRROR_UNIT_NAME, UNIT_STOP_START_CONFIG)?;

        println!("end install");
    }

    Ok(())
}

const SCRIPTS_DIR: &str = "scripts";
const SYSTEMD_DIR: &str = "systemd";
const SCRIPT_NAME: &str = "calendar_mirror_server.py";
const CALENDAR_MIRROR_UNIT_NAME: &str = "calendar_mirror.service";
const NTP_UNIT_NAME: &str = "ntp.service";
const UNIT_STOP_START_CONFIG: &str = "replace";
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_VAR_DIR: &str = ".";
const VAR_DIR_FS_TYPE: &str = "ext4";
const CALENDAR_MIRROR_VAR: &str = "CALENDAR_MIRROR_VAR";
const CALENDAR_MIRROR_DEV: &str = "CALENDAR_MIRROR_DEV";
const NETWORK_CHECK_POLL_PERIOD: Duration = Duration::from_millis(750);

fn sync_time<'a>(system_d: &'a dbus::ConnPath<'a, &'a dbus::Connection>) -> Result<(), Error> {
    match system_d.stop_unit(NTP_UNIT_NAME, UNIT_STOP_START_CONFIG) {
        Ok(_) => {
            println!("stopped the unit {:?}", NTP_UNIT_NAME);
        }
        Err(err) => {
            println!(
                "error disabling {:?}. {:?} was probably not running",
                err, NTP_UNIT_NAME
            );
        }
    };

    loop {
        let output = Command::new("sntp")
            .arg("-S")
            .arg("0.us.pool.ntp.org")
            .output()?;
        println!("status: {}", output.status);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

        if output.status.success() {
            break;
        }
        thread::sleep(NETWORK_CHECK_POLL_PERIOD);
    }
    println!("after sntp");
    system_d.start_unit(NTP_UNIT_NAME, UNIT_STOP_START_CONFIG)?;
    Ok(())
}

fn main() -> Result<(), Error> {
    let path_opt = var_os("PATH");
    let paths = if let Some(ref val) = path_opt {
        val.clone().into_string().expect("invalid path")
    } else {
        "".to_string()
    };
    println!("path: {}", paths);

    let dbus = Connection::get_private(BusType::System)?;
    let system_d = system_d_inst(&dbus)?;
    let args: Vec<String> = env::args().collect();
    let dest_base: &Path = Path::new("/opt/");
    for arg in args.iter() {
        match arg.as_str() {
            "--install" => {
                installation(&system_d, PackageAction::Install, dest_base, VERSION)?;
                process::exit(0);
            }
            "--uninstall" => {
                installation(&system_d, PackageAction::Uninstall, &dest_base, VERSION)?;
                process::exit(0);
            }
            _ => {}
        }
    }

    let quitter = Arc::new(AtomicBool::new(false));

    if cfg!(feature = "render_stm") {
        cal_machine::render_stms()?;
    } else {
        println!("before sync time");
        sync_time(&system_d)?;
        println!("after sync time");

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

                let var_dir_opt = var_os(CALENDAR_MIRROR_VAR);
                let var_dir_os = &var_dir_opt.clone().unwrap_or(DEFAULT_VAR_DIR.into());
                let var_dir: &Path = Path::new(var_dir_os);

                let var_dir_fs_type: &Path = Path::new(VAR_DIR_FS_TYPE);
                let mut base_flags = MsFlags::empty();
                base_flags.insert(MsFlags::MS_NOATIME);
                base_flags.insert(MsFlags::MS_NOSUID);
                base_flags.insert(MsFlags::MS_NODEV);

                let mut ro_flags = base_flags.clone();
                ro_flags.insert(MsFlags::MS_RDONLY);

                if var_dir_opt.is_some() {
                    let var_dir_dev_opt = var_os(CALENDAR_MIRROR_DEV);
                    let var_dir_dev_os= &var_dir_dev_opt.clone().expect(format!("If the var mount point is specified by the environment so too must a block device using the {} environment variable", CALENDAR_MIRROR_DEV).as_str());
                    let var_dir_dev: &Path = Path::new(var_dir_dev_os);

                    create_dir_all(var_dir)?;
                    println!(
                        "before mount: {:?} flags: {:?} dev: {:?} fs type: {:?}",
                        var_dir.display(),
                        ro_flags,
                        var_dir_dev,
                        var_dir_fs_type
                    );
                    mount(
                        Option::<&Path>::Some(var_dir_dev),
                        var_dir,
                        Option::<&Path>::Some(var_dir_fs_type),
                        ro_flags,
                        Option::<&Path>::None,
                    )?;
                    println!("after mount");
                }
                base_flags.insert(MsFlags::MS_REMOUNT);
                let rw_flags = base_flags;
                ro_flags.insert(MsFlags::MS_REMOUNT);
                let ro_flags = ro_flags;

                let config_file = var_dir.join(Path::new("refresh.json"));

                let simple_saver = |refresh_token: &RefreshToken, renderer: &mut Renderer| {
                    renderer.display_save_warning()?;
                    if var_dir_opt.is_some() {
                        println!("remounting rw and saving refresh token");
                        mount(
                            Option::<&Path>::None,
                            var_dir,
                            Option::<&Path>::None,
                            rw_flags,
                            Option::<&Path>::None,
                        )?;
                        refresh_token.save(&config_file)?;
                        mount(
                            Option::<&Path>::None,
                            var_dir,
                            Option::<&Path>::None,
                            ro_flags,
                            Option::<&Path>::None,
                        )?;
                        println!("remounting ro");
                    } else {
                        println!("saving refresh token");
                        refresh_token.save(&config_file)?;
                    }
                    Ok(())
                };

                loop {
                    match cal_machine::run(&mut renderer, &quitter, &config_file, simple_saver) {
                        Err(cal_machine::Error::Reqwest(error)) => {
                            eprintln!("reqwest error: {:?}", error);
                            thread::sleep(Duration::from_secs(5));
                        }
                        Err(error) => {
                            renderer.clear()?;
                            return Err(error.into());
                        }
                        Ok(()) => {
                            break;
                        }
                    }
                }

                if var_dir_opt.is_some() {
                    println!("before umount: {:?}", var_dir.display(),);
                    umount(var_dir)?;
                    println!("after umount");
                }

                println!("finishing up");
            }
            ForkResult::Child => {
                println!("child will now start server...");
                execvp(
                    &CString::new(SCRIPT_NAME).expect(&format!("Invalid CString: {}", SCRIPT_NAME)),
                    &[],
                )?;
            }
        }
    }
    Ok(())
}
