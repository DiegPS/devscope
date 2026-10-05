"""Real-terminal lifecycle tests using only Python's standard library.

Run after cargo test --no-run, or pass --binary PATH for an instrumented unit
test executable. The child runs a test-only entry point with owned config.
"""
import argparse
import contextlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time


class Session:
    def __init__(self):
        self.output = bytearray()
        self.changed = threading.Condition()
        self.reader = None

    def append(self, data):
        with self.changed:
            self.output.extend(data)
            self.changed.notify_all()
        if b"\x1b[6n" in data:
            self.send(b"\x1b[1;1R")

    def expect(self, text, timeout=15):
        deadline = time.monotonic() + timeout
        with self.changed:
            while text not in self.output:
                if self.poll() is not None:
                    raise AssertionError(f"child exited before {text!r}: {bytes(self.output)[-500:]!r}")
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise AssertionError(f"terminal timed out waiting for {text!r}")
                self.changed.wait(min(remaining, .05))


@contextlib.contextmanager
def posix_session(command, env):
    import fcntl
    import pty
    import struct
    import termios

    master, slave = pty.openpty()
    session = Session()
    initial = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 140, 0, 0))
    process = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave, env={**env, "TERM": "xterm-256color"})
    session.poll = process.poll
    session.wait = lambda: process.wait(timeout=10)
    session.send = lambda data: os.write(master, data)
    def read():
        try:
            while data := os.read(master, 16384):
                session.append(data)
        except OSError:
            pass
    session.reader = threading.Thread(target=read, daemon=True)
    session.reader.start()
    try:
        yield session
        assert termios.tcgetattr(slave) == initial, "raw terminal settings were not restored"
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)
        os.close(slave)
        session.reader.join(timeout=2)
        os.close(master)


@contextlib.contextmanager
def windows_session(command, env):
    import ctypes as c
    from ctypes import wintypes as w
    kernel = c.WinDLL("kernel32", use_last_error=True)
    def api(name, args, result=w.BOOL):
        func = getattr(kernel, name)
        func.argtypes, func.restype = args, result
        return func
    def check(value):
        if not value:
            raise c.WinError(c.get_last_error())
    class Coord(c.Structure):
        _fields_ = [("x", c.c_short), ("y", c.c_short)]
    class Startup(c.Structure):
        _fields_ = [("cb", w.DWORD), ("reserved", w.LPWSTR), ("desktop", w.LPWSTR), ("title", w.LPWSTR),
                    ("x", w.DWORD), ("y", w.DWORD), ("width", w.DWORD), ("height", w.DWORD),
                    ("chars_x", w.DWORD), ("chars_y", w.DWORD), ("fill", w.DWORD), ("flags", w.DWORD),
                    ("show", w.WORD), ("reserved_size", w.WORD), ("reserved_data", c.c_void_p),
                    ("stdin", w.HANDLE), ("stdout", w.HANDLE), ("stderr", w.HANDLE)]
    class StartupEx(c.Structure):
        _fields_ = [("startup", Startup), ("attributes", c.c_void_p)]
    class Process(c.Structure):
        _fields_ = [("process", w.HANDLE), ("thread", w.HANDLE), ("pid", w.DWORD), ("tid", w.DWORD)]
    pipe = api("CreatePipe", [c.POINTER(w.HANDLE), c.POINTER(w.HANDLE), c.c_void_p, w.DWORD])
    create_console = api("CreatePseudoConsole", [Coord, w.HANDLE, w.HANDLE, w.DWORD, c.POINTER(w.HANDLE)], c.c_long)
    close_console = api("ClosePseudoConsole", [w.HANDLE], None)
    close = api("CloseHandle", [w.HANDLE])
    initialize = api("InitializeProcThreadAttributeList", [c.c_void_p, w.DWORD, w.DWORD, c.POINTER(c.c_size_t)])
    update = api("UpdateProcThreadAttribute", [c.c_void_p, w.DWORD, c.c_size_t, c.c_void_p, c.c_size_t, c.c_void_p, c.c_void_p])
    delete = api("DeleteProcThreadAttributeList", [c.c_void_p], None)
    create_process = api("CreateProcessW", [w.LPCWSTR, w.LPWSTR, c.c_void_p, c.c_void_p, w.BOOL, w.DWORD, c.c_void_p, w.LPCWSTR, c.POINTER(StartupEx), c.POINTER(Process)])
    read_file = api("ReadFile", [w.HANDLE, c.c_void_p, w.DWORD, c.POINTER(w.DWORD), c.c_void_p])
    write_file = api("WriteFile", [w.HANDLE, c.c_void_p, w.DWORD, c.POINTER(w.DWORD), c.c_void_p])
    wait = api("WaitForSingleObject", [w.HANDLE, w.DWORD], w.DWORD)
    exit_code = api("GetExitCodeProcess", [w.HANDLE, c.POINTER(w.DWORD)])
    terminate = api("TerminateProcess", [w.HANDLE, w.UINT])
    get_std = api("GetStdHandle", [w.DWORD], w.HANDLE)
    set_std = api("SetStdHandle", [w.DWORD, w.HANDLE])
    ri, wi, ro, wo, console = [w.HANDLE() for _ in range(5)]
    proc = Process()
    attrs = None
    initialized = False
    session = Session()
    try:
        check(pipe(c.byref(ri), c.byref(wi), None, 0))
        check(pipe(c.byref(ro), c.byref(wo), None, 0))
        status = create_console(Coord(140, 40), ri, wo, 0, c.byref(console))
        if status:
            raise RuntimeError(f"CreatePseudoConsole failed: {status}")
        size = c.c_size_t()
        initialize(None, 1, 0, c.byref(size))
        attrs = c.create_string_buffer(size.value)
        check(initialize(attrs, 1, 0, c.byref(size)))
        initialized = True
        check(update(attrs, 0, 0x00020016, console, c.sizeof(w.HANDLE), None, None))
        startup = StartupEx()
        startup.startup.cb = c.sizeof(StartupEx)
        startup.attributes = c.cast(attrs, c.c_void_p)
        cmd = c.create_unicode_buffer(subprocess.list2cmdline(command))
        env_block = c.create_unicode_buffer("\0".join(f"{key}={value}" for key, value in sorted(env.items(), key=lambda pair: pair[0].upper())) + "\0\0")
        saved = [get_std(index) for index in (-10, -11, -12)]
        try:
            # Redirected parent handles must not bypass the pseudoconsole.
            for index in (-10, -11, -12):
                set_std(index, None)
            check(create_process(command[0], cmd, None, None, False, 0x00080000 | 0x00000400, env_block, None, c.byref(startup), c.byref(proc)))
        finally:
            for index, handle in zip((-10, -11, -12), saved):
                set_std(index, handle)
        close(ri); ri = w.HANDLE()
        close(wo); wo = w.HANDLE()
        close(proc.thread); proc.thread = None
        def code():
            value = w.DWORD()
            check(exit_code(proc.process, c.byref(value)))
            return value.value
        session.poll = lambda: None if wait(proc.process, 0) == 258 else code()
        def wait_child():
            if wait(proc.process, 10000) != 0:
                raise TimeoutError("TUI did not exit")
            return code()
        session.wait = wait_child
        def send(data):
            written = w.DWORD()
            check(write_file(wi, data, len(data), c.byref(written), None))
            assert written.value == len(data)
        session.send = send
        def read():
            buffer, count = c.create_string_buffer(16384), w.DWORD()
            while read_file(ro, buffer, len(buffer), c.byref(count), None) and count.value:
                session.append(buffer.raw[:count.value])
        session.reader = threading.Thread(target=read, daemon=True)
        session.reader.start()
        yield session
    finally:
        if proc.process:
            if wait(proc.process, 0) != 0:
                terminate(proc.process, 1)
                wait(proc.process, 10000)
            close(proc.process)
        if wi: close(wi)
        if console: close_console(console)
        if session.reader: session.reader.join(timeout=2)
        for handle in (ri, ro, wo, proc.thread):
            if handle: close(handle)
        if initialized: delete(attrs)


def test_binary():
    result = subprocess.run(["cargo", "test", "--locked", "--bin", "ds", "--no-run", "--message-format=json"], check=True, capture_output=True, text=True)
    for line in result.stdout.splitlines():
        artifact = json.loads(line)
        if artifact.get("reason") == "compiler-artifact" and artifact.get("profile", {}).get("test") and artifact.get("executable"):
            return artifact["executable"]
    raise RuntimeError("unit test executable not found")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary")
    args = parser.parse_args()
    binary = str(Path(args.binary or test_binary()).resolve())
    session_factory = windows_session if os.name == "nt" else posix_session
    scenarios = [
        ("quit", [b"q"]),
        ("search then cancel", [b"/qq", b"\x1b", b"q"]),
        ("menu then cancel", [b"o", b"\x1b", b"q"]),
        ("help scroll then close", [b"?", b"\x1b[F", b"\x1b", b"q"]),
        ("details focus scroll then return", [b"\t", b"\x1b[F", b"\t", b"q"]),
        ("menu navigate then cancel", [b"o", b"\x1b[F", b"\x1b", b"q"]),
        ("reload then quit", [b"rq"]),
        ("repeated reload then quit", [b"rrrq"]),
        ("Ctrl+C", [b"\x03"]),
    ]
    for name, keys in scenarios:
        with tempfile.TemporaryDirectory(prefix="ds-tui-test-") as root:
            Path(root, "Cargo.toml").write_text("[package]\nname='fixture'\nversion='0.1.0'\n", encoding="utf-8")
            env = {**os.environ, "DS_TEST_TUI_ROOT": root}
            command = [binary, "--ignored", "--exact", "tui::tests::real_terminal_child", "--nocapture", "--test-threads=1"]
            with session_factory(command, env) as session:
                session.expect(b"quit")
                for part in keys:
                    session.send(part)
                    if part != keys[-1]:
                        time.sleep(.1)
                        assert session.poll() is None, "mode transition unexpectedly exited"
                    if name == "help scroll then close" and part == b"\x1b[F":
                        session.expect(b"Ctrl+C")
                    if name == "details focus scroll then return" and part == b"\x1b[F":
                        session.expect(b"Health")
                assert session.wait() == 0, bytes(session.output)[-500:]
                # Output can still be draining after process termination.
                deadline = time.monotonic() + 2
                with session.changed:
                    while b"\x1b[?1049l" not in session.output and time.monotonic() < deadline:
                        session.changed.wait(.05)
                assert b"\x1b[?1049h" in session.output
                assert b"\x1b[?1049l" in session.output, "alternate screen not restored"
                assert b"\x1b[?25h" in session.output, "cursor not restored"
                assert not Path(root, "test-config.toml").exists(), "quit must not save config"
            print(f"PASS: {name}")


if __name__ == "__main__":
    main()
