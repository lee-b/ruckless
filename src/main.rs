// A simple port of Suckless Tools' sinit[1] to Rust[2]
//
// [1] http://git.suckless.org/sinit
// [2] https://www.rust-lang.org
//
// Author: Lee Braiden <leebraid@gmail.com>
//

extern crate libc;
use std::ffi::CString;
use std::mem;
use std::ptr;


struct Config {
    startup_command: Vec<String>,
    shutdown_command: Vec<String>,
    reboot_command: Vec<String>,
}

fn get_libc_error() -> String {
    let err_desc =
        unsafe { std::ffi::CString::from_raw(libc::strerror(*libc::__errno_location())) };

    err_desc.into_string().unwrap_or("unknown system error".to_string())
}

fn child_proc(args: Vec<CString>, sig_set: &libc::sigset_t) -> Result<libc::pid_t, String> {
    // successfully forked; in child process now

    let res = unsafe { libc::pthread_sigmask(libc::SIG_UNBLOCK, sig_set, ptr::null_mut()) };

    if res != 0 {
        Err("pthread_sigmask call failed: ".to_string() + get_libc_error().as_ref())

    } else {
        let err_desc = get_libc_error();
        if unsafe { libc::setsid() } == -1 {
            Err("setsid call failed: ".to_string() + err_desc.as_ref())

        } else {
            let mut new_args = args.iter().map(|arg| arg.as_ptr()).collect::<Vec<*const i8>>();
            new_args.push(ptr::null());
            new_args.shrink_to_fit();

            unsafe { libc::execvp(args[0].as_ptr() as *const i8, &new_args[0]) };

            // If we reach here, execvp failed, so handle the error.
            // the parent function call is proceeding in parallel, and
            // probably already completed, so just

	    let msg = format!("ERROR: couldn't exec child process {}: {}", args.into_iter().map(|cs_arg| cs_arg.into_string().unwrap()).collect::<Vec<String>>().join(" "), get_libc_error());
  	    Err(msg)
        }

    }
}

fn spawn(args: &Vec<String>, sig_set: &libc::sigset_t) -> Result<libc::pid_t, String> {
    match unsafe { libc::fork() } {
        0 => {
            child_proc(args.into_iter()
                           .map(|arg| CString::new(arg.to_owned()).unwrap())
                           .collect::<Vec<CString>>(),
                       sig_set)
        }
        -1 => Err(get_libc_error()),
        pid => Ok(pid),
    }
}

fn do_cmd(cmd_args: &Vec<String>, sig_set: &libc::sigset_t) -> bool {
    if let Err(e) = spawn(&cmd_args, sig_set) {
        println!("{}", e);
        false
    } else {
        true
    }
}

fn sigreap() -> bool {
    loop {
        let res: i32;
        unsafe {
            res = libc::waitpid(-1, ptr::null_mut(), libc::WNOHANG);
        }

        if res > 0 {
            break;
        }
    }

    true
}

#[cfg(not(debug_assertions))]
fn build_config() -> Config {
    // Run commands in the /bin directory in release builds
    Config {
        startup_command: vec![ "/bin/rc.init".to_string(), ],
        shutdown_command: vec![ "/bin/rc.shutdown".to_string(), "poweroff".to_string(), ],
        reboot_command: vec![ "/bin/rc.shutdown".to_string(), "reboot".to_string(), ],
    }
}

#[cfg(debug_assertions)]
fn build_config() -> Config {
    // Run commands in the local directory in debug builds
    Config {
        startup_command: vec![ "debug_init_scripts/rc.init".to_string(), ],
        shutdown_command: vec![ "debug_init_scripts/rc.shutdown".to_string(), "poweroff".to_string(), ],
        reboot_command: vec![ "debug_init_scripts/rc.shutdown".to_string(), "reboot".to_string(), ],
    }
}

fn main() {
    let config = build_config();

    #[cfg(build="release")]    {
        if 1 != unsafe { libc::getpid() } {
            println!("ERROR: attempted to run init as a pid other than 0!");
            std::process::exit(1i32);
        }
    }

    let mut sig_set: libc::sigset_t = unsafe { mem::zeroed() };
    unsafe {
        let sig_set_ptr = &mut sig_set as *mut libc::sigset_t;

        libc::sigfillset(sig_set_ptr);
        libc::pthread_sigmask(libc::SIG_BLOCK, sig_set_ptr, ptr::null_mut());
    }

    println!("Init: begin.");

    if let false = do_cmd(&config.startup_command, &sig_set) {
        std::process::exit(1i32);
    }

    println!("Init: up and running.");
 
    let mut sig = 0i32;

    loop {
        unsafe {
            libc::sigwait(&mut sig_set as *mut libc::sigset_t, &mut sig as *mut i32);
        }

        let _ = match sig {
            libc::SIGCHLD => sigreap(),
            libc::SIGUSR1 => do_cmd(&config.shutdown_command, &sig_set),
            libc::SIGINT => do_cmd(&config.reboot_command, &sig_set),
            _ => true,
        };
    }
}
