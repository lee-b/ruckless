extern crate libc;
use std::mem;
use std::ptr;

enum ShutdownMode {
    Reboot,
    PowerOff,
}

fn call_rc_shutdown(mode: ShutdownMode) {
    let mode_str = match mode {
        ShutdownMode::Reboot => "reboot",
        ShutdownMode::PowerOff => "poweroff",
    };

    let cmd_str = "/bin/rc.shutdown";
    let cmd_res = std::process::Command::new(cmd_str).arg(mode_str).spawn();

    if let Err(e) = cmd_res {
        println!("Couldn't spawn /bin/rc.shutdown: {:?}", e);
    }
}

fn sigpoweroff() {
    call_rc_shutdown(ShutdownMode::PowerOff)
}

fn sigreap() {
    loop {
        let res: i32;
        unsafe {
            res = libc::waitpid(-1, ptr::null_mut(), libc::WNOHANG);
        }

        if res > 0 {
            break;
        }
    }
}

fn sigreboot() {
    call_rc_shutdown(ShutdownMode::Reboot)
}

fn main() {
    let pid: i32;

    unsafe {
        pid = libc::getpid();
    }

    if pid != 1 {
        println!("ERROR: attempted to run init as a pid other than 0!");
        //        std::process::exit(1i32);
    }

    let mut sig_set: libc::sigset_t = unsafe { mem::zeroed() };
    unsafe {
        #[allow(uninitialised)]
        let sig_set_ptr = &mut sig_set as *mut libc::sigset_t;

        libc::sigfillset(sig_set_ptr);
        libc::pthread_sigmask(libc::SIG_BLOCK, sig_set_ptr, ptr::null_mut());
    }

    let cmd_res = std::process::Command::new("/bin/rc.init").spawn();
    if let Err(e) = cmd_res {
        println!("ERROR: Couldn't spawn /bin/rc.init: {:?}", e);
        std::process::exit(1i32);
    }

    println!("Running.");

    let mut sig = 0i32;

    loop {
        unsafe {
            libc::sigwait(&mut sig_set as *mut libc::sigset_t, &mut sig as *mut i32);
        }

        match sig {
            libc::SIGUSR1 => sigpoweroff(),
            libc::SIGCHLD => sigreap(),
            libc::SIGINT => sigreboot(),
            _ => {}
        }
    }
}
