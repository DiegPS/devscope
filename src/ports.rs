use std::collections::HashMap;

use listeners::Protocol;

pub fn detect_project_ports(project_paths: &[String]) -> HashMap<String, Vec<u16>> {
    let all = match listeners::get_all() {
        Ok(list) => list,
        Err(_) => return HashMap::new(),
    };

    let mut result: HashMap<String, Vec<u16>> = HashMap::new();

    for listener in &all {
        if listener.protocol != Protocol::TCP {
            continue;
        }

        let port = listener.socket.port();
        if port == 0 {
            continue;
        }

        if let Some(cmd) = get_process_cmd(listener.process.pid) {
            if let Some(path) = find_matching_project(&cmd, project_paths) {
                let ports = result.entry(path).or_default();
                if !ports.contains(&port) {
                    ports.push(port);
                    ports.sort_unstable();
                }
            }
        }
    }

    result
}

fn find_matching_project(cmd: &str, project_paths: &[String]) -> Option<String> {
    for path in project_paths {
        if cmd_contains_path(cmd, path) {
            return Some(path.clone());
        }
    }
    None
}

fn cmd_contains_path(cmd: &str, path: &str) -> bool {
    let cmd_norm = cmd.replace('\\', "/");

    let mut path_norm = path.replace('\\', "/");
    if path_norm.ends_with('/') {
        path_norm.pop();
    }

    #[cfg(windows)]
    let (cmd_norm, path_norm) = (cmd_norm.to_lowercase(), path_norm.to_lowercase());

    if let Some(pos) = cmd_norm.find(&path_norm) {
        let after = pos + path_norm.len();
        if after >= cmd_norm.len() {
            return true;
        }
        let next = cmd_norm.as_bytes()[after] as char;
        return next == '/' || next == '\\' || next == ' ' || next == '\0' || next == '\n';
    }

    false
}

// ── Linux ────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn get_process_cmd(pid: u32) -> Option<String> {
    let path = format!("/proc/{}/cmdline", pid);
    let data = std::fs::read(path).ok()?;
    if data.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&data).replace('\0', " "))
}

// ── Windows ──────────────────────────────────────────────────────────────

#[cfg(windows)]
fn get_process_cmd(pid: u32) -> Option<String> {
    const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
    const PROCESS_VM_READ: u32 = 0x0010;
    const PROCESS_CMD_LINE_INFO: u32 = 60;

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        _maximum_length: u16,
        buffer: *mut u16,
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn NtQueryInformationProcess(
            process_handle: isize,
            process_information_class: u32,
            process_information: *mut std::ffi::c_void,
            process_information_length: u32,
            return_length: *mut u32,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(dw_desired_access: u32, b_inherit_handle: i32, dw_process_id: u32) -> isize;

        fn CloseHandle(h_object: isize) -> i32;
    }

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
        if handle == 0 || handle == -1 {
            return None;
        }

        let mut return_len: u32 = 0;
        let us_size = std::mem::size_of::<UnicodeString>() as u32;

        NtQueryInformationProcess(
            handle,
            PROCESS_CMD_LINE_INFO,
            std::ptr::null_mut(),
            0,
            &mut return_len,
        );

        if return_len < us_size {
            CloseHandle(handle);
            return None;
        }

        let mut buf: Vec<u8> = vec![0u8; return_len as usize];
        let status = NtQueryInformationProcess(
            handle,
            PROCESS_CMD_LINE_INFO,
            buf.as_mut_ptr() as *mut std::ffi::c_void,
            return_len,
            &mut return_len,
        );

        CloseHandle(handle);

        if status < 0 {
            return None;
        }

        let us = &*(buf.as_ptr() as *const UnicodeString);
        if us.buffer.is_null() || us.length == 0 {
            return None;
        }

        let len = (us.length as usize / 2).min(4096);
        let wide = std::slice::from_raw_parts(us.buffer, len);
        Some(String::from_utf16_lossy(wide))
    }
}

// ── macOS ────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn get_process_cmd(pid: u32) -> Option<String> {
    const CTL_KERN: std::os::raw::c_int = 1;
    const KERN_PROCARGS2: std::os::raw::c_int = 49;

    extern "C" {
        fn sysctl(
            name: *mut std::os::raw::c_int,
            namelen: u32,
            oldp: *mut std::os::raw::c_void,
            oldlenp: *mut usize,
            newp: *mut std::os::raw::c_void,
            newlen: usize,
        ) -> std::os::raw::c_int;
    }

    unsafe {
        let mut mib = [CTL_KERN, KERN_PROCARGS2, pid as std::os::raw::c_int];
        let mut size: usize = 0;

        if sysctl(
            mib.as_mut_ptr(),
            3,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        ) != 0
            || size == 0
        {
            return None;
        }

        let mut buf: Vec<u8> = vec![0; size];
        if sysctl(
            mib.as_mut_ptr(),
            3,
            buf.as_mut_ptr() as *mut std::ffi::c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }

        if buf.len() < 4 {
            return None;
        }

        let argc = i32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        if argc == 0 {
            return None;
        }

        let exec_start = 4;
        let exec_end = buf[exec_start..].iter().position(|&b| b == 0)?;
        let exec_path = std::str::from_utf8(&buf[exec_start..exec_start + exec_end]).ok()?;

        let mut pos = exec_start + exec_end + 1;
        while pos < buf.len() && buf[pos] == 0 {
            pos += 1;
        }

        let mut args: Vec<String> = vec![exec_path.to_string()];
        for _ in 1..argc {
            if pos >= buf.len() {
                break;
            }
            let arg_end = buf[pos..].iter().position(|&b| b == 0)?;
            args.push(
                std::str::from_utf8(&buf[pos..pos + arg_end])
                    .ok()?
                    .to_string(),
            );
            pos += arg_end + 1;
        }

        Some(args.join(" "))
    }
}

// ── Fallback (other platforms) ───────────────────────────────────────────

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn get_process_cmd(_pid: u32) -> Option<String> {
    None
}
