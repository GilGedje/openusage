//! Starts `ccline refresh` as a detached background process so the status line stays instant.

use std::process::{Command, Stdio};

use usage_core::log;

pub fn refresh_detached() {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return log::error("spawn", &e.to_string()),
    };
    let mut cmd = Command::new(exe);
    cmd.arg("refresh").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    // Own process group / no console, so Claude Code cancelling the status line doesn't kill it.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }
    if let Err(e) = cmd.spawn() {
        log::error("spawn", &e.to_string());
    }
}
