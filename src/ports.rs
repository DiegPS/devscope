use std::collections::HashMap;

use listeners::Protocol;

pub fn detect_project_ports(project_paths: &[String]) -> HashMap<String, Vec<u16>> {
    if project_paths.is_empty() {
        return HashMap::new();
    }
    let all = match listeners::get_all() {
        Ok(list) => list,
        Err(_) => return HashMap::new(),
    };

    let listeners = all
        .iter()
        .filter(|listener| listener.protocol == Protocol::TCP)
        .map(|listener| (listener.process.pid, listener.socket.port()));
    associate_project_ports(listeners, project_paths, get_process_cmd)
}

fn associate_project_ports(
    listeners: impl IntoIterator<Item = (u32, u16)>,
    project_paths: &[String],
    mut read_command: impl FnMut(u32) -> Option<String>,
) -> HashMap<String, Vec<u16>> {
    let mut result: HashMap<String, Vec<u16>> = HashMap::new();
    // Cache matches, including misses, only for this detection pass. A process
    // with several listeners needs one command-line query and one path match.
    let mut projects_by_pid = HashMap::new();
    for (pid, port) in listeners {
        if port == 0 {
            continue;
        }

        let path = projects_by_pid.entry(pid).or_insert_with(|| {
            read_command(pid).and_then(|cmd| find_matching_project(&cmd, project_paths))
        });
        if let Some(path) = path {
            let ports = result.entry(path.clone()).or_default();
            if !ports.contains(&port) {
                ports.push(port);
            }
        }
    }
    for ports in result.values_mut() {
        ports.sort_unstable();
    }
    result
}

fn find_matching_project(cmd: &str, project_paths: &[String]) -> Option<String> {
    project_paths
        .iter()
        .filter(|path| cmd_contains_path(cmd, path))
        .max_by_key(|path| path.len())
        .cloned()
}

fn cmd_contains_path(cmd: &str, path: &str) -> bool {
    let cmd_norm = cmd.replace('\\', "/");

    let mut path_norm = path.replace('\\', "/");
    if path_norm.ends_with('/') && path_norm.len() > 1 {
        path_norm.pop();
    }
    if path_norm.is_empty() {
        return false;
    }

    #[cfg(windows)]
    let (cmd_norm, path_norm) = (cmd_norm.to_lowercase(), path_norm.to_lowercase());

    cmd_norm.match_indices(&path_norm).any(|(pos, _)| {
        let before = cmd_norm[..pos].chars().next_back();
        if before.is_some_and(|ch| {
            !ch.is_whitespace() && ch != '\0' && ch != '"' && ch != '\'' && ch != '='
        }) {
            return false;
        }
        let after = pos + path_norm.len();
        cmd_norm[after..].chars().next().is_none_or(|ch| {
            ch == '/' || ch.is_whitespace() || ch == '\0' || ch == '"' || ch == '\''
        })
    })
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

    struct ProcessHandle(isize);
    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            // SAFETY: this guard owns the valid handle returned by OpenProcess.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    // SAFETY: integer arguments contain documented access flags and a process ID.
    let raw_handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
    if raw_handle == 0 || raw_handle == -1 {
        return None;
    }
    let handle = ProcessHandle(raw_handle);

    // SAFETY: the first call queries size only. The second receives a writable,
    // pointer-aligned allocation of at least the requested byte count. Handles
    // and buffers remain alive for both calls; NTSTATUS is checked before parsing.
    unsafe {
        let mut return_len: u32 = 0;
        let us_size = std::mem::size_of::<CommandLineHeader>() as u32;

        NtQueryInformationProcess(
            handle.0,
            PROCESS_CMD_LINE_INFO,
            std::ptr::null_mut(),
            0,
            &mut return_len,
        );

        if return_len < us_size || return_len > 1024 * 1024 {
            return None;
        }

        let byte_len = return_len as usize;
        let mut buf = vec![0usize; byte_len.div_ceil(std::mem::size_of::<usize>())];
        let status = NtQueryInformationProcess(
            handle.0,
            PROCESS_CMD_LINE_INFO,
            buf.as_mut_ptr() as *mut std::ffi::c_void,
            return_len,
            &mut return_len,
        );

        if status < 0 || return_len as usize > byte_len {
            return None;
        }

        // SAFETY: initialized usize storage has no padding and covers byte_len
        // bytes in a single allocation; u8 has alignment 1. Borrow ends before buf.
        let bytes = std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), byte_len);
        decode_windows_command_line(bytes)
    }
}

#[cfg(any(windows, test))]
#[repr(C)]
struct CommandLineHeader {
    length: u16,
    maximum_length: u16,
    buffer: *const u16,
}

#[cfg(any(windows, test))]
fn decode_windows_command_line(bytes: &[u8]) -> Option<String> {
    if bytes.len() < std::mem::size_of::<CommandLineHeader>() {
        return None;
    }
    // SAFETY: the entire header fits within bytes. read_unaligned imposes no
    // alignment requirement and the fields are integers/a raw pointer, not refs.
    let header = unsafe { bytes.as_ptr().cast::<CommandLineHeader>().read_unaligned() };
    let length = usize::from(header.length);
    if header.buffer.is_null()
        || length == 0
        || length % 2 != 0
        || header.length > header.maximum_length
    {
        return None;
    }
    let offset = (header.buffer as usize).checked_sub(bytes.as_ptr() as usize)?;
    if offset < std::mem::size_of::<CommandLineHeader>() {
        return None;
    }
    let content = bytes.get(offset..offset.checked_add(length)?)?;
    let wide: Vec<u16> = content
        .chunks_exact(2)
        .take(4096)
        .map(|pair| u16::from_ne_bytes([pair[0], pair[1]]))
        .collect();
    Some(String::from_utf16_lossy(&wide))
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

        parse_macos_command_line(buf.get(..size)?)
    }
}

#[cfg(any(target_os = "macos", test))]
fn parse_macos_command_line(bytes: &[u8]) -> Option<String> {
    let argc = usize::try_from(i32::from_ne_bytes(bytes.get(..4)?.try_into().ok()?)).ok()?;
    if argc == 0 || argc > bytes.len() {
        return None;
    }
    let exec_end = 4 + bytes.get(4..)?.iter().position(|&b| b == 0)?;
    let exec_path = std::str::from_utf8(&bytes[4..exec_end]).ok()?;
    let mut pos = exec_end + 1;
    while bytes.get(pos) == Some(&0) {
        pos += 1;
    }
    let mut args = vec![exec_path.to_string()];
    for index in 0..argc {
        let end = pos + bytes.get(pos..)?.iter().position(|&b| b == 0)?;
        let arg = std::str::from_utf8(&bytes[pos..end]).ok()?;
        if index > 0 {
            args.push(arg.to_string());
        }
        pos = end + 1;
    }
    Some(args.join(" "))
}

// ── Fallback (other platforms) ───────────────────────────────────────────

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn get_process_cmd(_pid: u32) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listeners_share_command_queries_and_do_not_cache_across_scans() {
        let paths = vec!["/work/app".into(), "/work/app/child".into()];
        let mut calls = HashMap::new();
        let result = associate_project_ports(
            [
                (1, 9000),
                (2, 8000),
                (1, 3000),
                (1, 9000),
                (2, 8001),
                (3, 0),
            ],
            &paths,
            |pid| {
                *calls.entry(pid).or_insert(0) += 1;
                (pid == 1).then(|| "node /work/app/child/server.js".into())
            },
        );
        assert_eq!(calls, HashMap::from([(1, 1), (2, 1)]));
        assert_eq!(
            result,
            HashMap::from([(paths[1].clone(), vec![3000, 9000])])
        );

        let result = associate_project_ports([(1, 7000)], &paths, |_| {
            Some("node /work/app/server.js".into())
        });
        assert_eq!(result, HashMap::from([(paths[0].clone(), vec![7000])]));
    }

    #[test]
    fn matches_quoted_paths_and_uses_the_most_specific_project() {
        assert!(cmd_contains_path("node \"C:/work/app\"", "C:/work/app"));
        assert!(!cmd_contains_path(
            "node C:/work/application/main.js",
            "C:/work/app"
        ));
        assert!(!cmd_contains_path(
            "node X:/prefix/C:/work/app/main.js",
            "C:/work/app"
        ));
        assert!(!cmd_contains_path("anything", ""));
        assert!(cmd_contains_path(
            "C:/work/application C:/work/app/main.js",
            "C:/work/app"
        ));
        let paths = vec!["C:/work/app".into(), "C:/work/app/child".into()];
        assert_eq!(
            find_matching_project("node C:/work/app/child/main.js", &paths),
            Some(paths[1].clone())
        );
    }

    fn command_buffer(text: &str) -> Vec<u8> {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let header_len = std::mem::size_of::<CommandLineHeader>();
        let mut bytes = vec![0u8; header_len + wide.len() * 2];
        for (pair, value) in bytes[header_len..].chunks_exact_mut(2).zip(&wide) {
            pair.copy_from_slice(&value.to_ne_bytes());
        }
        let header = CommandLineHeader {
            length: (wide.len() * 2) as u16,
            maximum_length: (wide.len() * 2) as u16,
            buffer: bytes[header_len..].as_ptr().cast::<u16>(),
        };
        // SAFETY: the destination covers a complete header; write_unaligned does
        // not require alignment, and no references to the header are created.
        unsafe {
            bytes
                .as_mut_ptr()
                .cast::<CommandLineHeader>()
                .write_unaligned(header);
        }
        bytes
    }

    #[test]
    fn windows_command_parser_validates_ranges_and_utf16() {
        let bytes = command_buffer("ds proyecto-日本");
        assert_eq!(
            decode_windows_command_line(&bytes).as_deref(),
            Some("ds proyecto-日本")
        );
        assert!(decode_windows_command_line(&bytes[..bytes.len() - 1]).is_none());
        assert!(decode_windows_command_line(&[]).is_none());
        let mut bytes = command_buffer("ds");
        bytes[..2].copy_from_slice(&3u16.to_ne_bytes());
        assert!(decode_windows_command_line(&bytes).is_none());
    }

    #[test]
    #[cfg(windows)]
    fn windows_can_read_its_own_process_command_line() {
        let command = get_process_cmd(std::process::id()).unwrap();
        let exe = std::env::current_exe().unwrap();
        assert!(command.contains(exe.file_name().unwrap().to_str().unwrap()));
    }

    #[test]
    fn macos_parser_keeps_the_last_argument_and_rejects_truncated_data() {
        let mut bytes = 3i32.to_ne_bytes().to_vec();
        bytes.extend_from_slice(b"/usr/bin/node\0\0node\0server.js\0/work/project\0ENV=value\0");
        assert_eq!(
            parse_macos_command_line(&bytes).as_deref(),
            Some("/usr/bin/node server.js /work/project")
        );
        assert!(parse_macos_command_line(&bytes[..bytes.len() - 11]).is_none());
        assert!(parse_macos_command_line(&[]).is_none());
    }
}
