"""
Wire-level tracer for the Python BenchMesh service (migration Phase 1).

Monkeypatches pyserial and the USBTMC transport's `os` module at runtime so that
every byte written to / read from an instrument is logged with timestamps, call
duration and the driver method that caused it. No repository source is modified.

Output: one JSONL file per device in the session directory. Event fields:
  t       wall-clock time (epoch seconds)
  mono    monotonic seconds since tracer install
  ev      open | close | tx | rx | ctl | error
  hex     payload bytes as hex (tx/rx)
  text    payload decoded latin-1 with escapes (for humans)
  req     bytes requested (rx)
  dur_ms  duration of the underlying call
  op      outermost driver method on the stack (e.g. poll_status, query_voltage)
  fn      innermost driver method on the stack (e.g. _query_output_voltage)
  thread  thread name
"""

import errno
import json
import os
import sys
import threading
import time

_TRANSPORT_HELPERS = {"write", "write_line", "read", "read_until_reol", "read_binary"}

_lock = threading.Lock()
_files = {}
_session_dir = None
_path_to_dev = {}
_t0 = time.monotonic()
_tolerate_pty_ctl = False


def _escape(data: bytes) -> str:
    return data.decode("latin-1").encode("unicode_escape").decode("ascii")


def _driver_context():
    """Return (outermost, innermost) driver method names on the current stack."""
    outer = inner = None
    f = sys._getframe(2)
    while f is not None:
        co = f.f_code
        if f"{os.sep}drivers{os.sep}" in co.co_filename and co.co_name not in _TRANSPORT_HELPERS:
            if inner is None:
                inner = co.co_name
            outer = co.co_name
        f = f.f_back
    return outer, inner


def _dev_for(path) -> str:
    key = str(path)
    dev = _path_to_dev.get(key) or _path_to_dev.get(os.path.realpath(key))
    return dev or ("unmapped_" + os.path.basename(key))


def _emit(dev: str, ev: str, **fields):
    op, fn = _driver_context()
    rec = {"t": time.time(), "mono": round(time.monotonic() - _t0, 6), "ev": ev,
           "op": op, "fn": fn, "thread": threading.current_thread().name}
    rec.update(fields)
    line = json.dumps(rec, separators=(",", ":"))
    with _lock:
        fh = _files.get(dev)
        if fh is None:
            fh = open(os.path.join(_session_dir, f"{dev}.jsonl"), "a", buffering=1)
            _files[dev] = fh
        fh.write(line + "\n")


def _payload(data: bytes) -> dict:
    return {"hex": data.hex(), "text": _escape(data), "len": len(data)}


def _patch_serial():
    import serial

    cls = serial.Serial
    orig_init, orig_write, orig_read = cls.__init__, cls.write, cls.read
    orig_close, orig_dtr, orig_rts = cls.close, cls.setDTR, cls.setRTS

    def __init__(self, *args, **kwargs):
        t = time.monotonic()
        try:
            orig_init(self, *args, **kwargs)
        except Exception as e:
            _emit(_dev_for(kwargs.get("port", args[0] if args else "?")), "error", where="open", error=repr(e))
            raise
        _emit(_dev_for(self.port), "open", transport="serial", port=self.port,
              baudrate=self.baudrate, bytesize=self.bytesize, parity=self.parity,
              stopbits=self.stopbits, timeout=self.timeout, xonxoff=self.xonxoff,
              rtscts=self.rtscts, dsrdtr=self.dsrdtr, dur_ms=round((time.monotonic() - t) * 1000, 3))

    def write(self, data):
        t = time.monotonic()
        n = orig_write(self, data)
        _emit(_dev_for(self.port), "tx", dur_ms=round((time.monotonic() - t) * 1000, 3), **_payload(bytes(data)))
        return n

    def read(self, size=1):
        t = time.monotonic()
        data = orig_read(self, size)
        _emit(_dev_for(self.port), "rx", req=size, dur_ms=round((time.monotonic() - t) * 1000, 3), **_payload(bytes(data)))
        return data

    def close(self):
        _emit(_dev_for(self.port), "close")
        return orig_close(self)

    def _ctl(self, line, value, orig):
        try:
            result = orig(self, value)
        except OSError as e:
            _emit(_dev_for(self.port), "ctl", line=line, value=bool(value), ok=False, errno=e.errno, error=repr(e))
            # Pseudo-terminals (simulators) have no modem-control lines: ENOTTY.
            if _tolerate_pty_ctl and e.errno == errno.ENOTTY:
                return None
            raise
        _emit(_dev_for(self.port), "ctl", line=line, value=bool(value), ok=True)
        return result

    def setDTR(self, value=True):
        return _ctl(self, "DTR", value, orig_dtr)

    def setRTS(self, value=True):
        return _ctl(self, "RTS", value, orig_rts)

    cls.__init__, cls.write, cls.read, cls.close = __init__, write, read, close
    cls.setDTR, cls.setRTS = setDTR, setRTS


class _TracingOs:
    """Proxy for the `os` module used by the USBTMC transport."""

    def __init__(self, real):
        self._real = real
        self._fd_path = {}

    def __getattr__(self, name):
        return getattr(self._real, name)

    def open(self, path, flags, *args):
        fd = self._real.open(path, flags, *args)
        self._fd_path[fd] = path
        _emit(_dev_for(path), "open", transport="usbtmc", device=path)
        return fd

    def write(self, fd, data):
        t = time.monotonic()
        n = self._real.write(fd, data)
        _emit(_dev_for(self._fd_path.get(fd, fd)), "tx", dur_ms=round((time.monotonic() - t) * 1000, 3), **_payload(bytes(data)))
        return n

    def read(self, fd, size):
        t = time.monotonic()
        dev = _dev_for(self._fd_path.get(fd, fd))
        try:
            data = self._real.read(fd, size)
        except OSError as e:
            _emit(dev, "error", where="read", req=size, errno=e.errno, error=repr(e),
                  dur_ms=round((time.monotonic() - t) * 1000, 3))
            raise
        _emit(dev, "rx", req=size, dur_ms=round((time.monotonic() - t) * 1000, 3), **_payload(bytes(data)))
        return data

    def close(self, fd):
        _emit(_dev_for(self._fd_path.pop(fd, fd)), "close")
        return self._real.close(fd)


def _patch_usbtmc():
    from benchmesh_service.transport import usbtmc

    usbtmc.os = _TracingOs(os)


def install(session_dir: str, devices: list, tolerate_pty_ctl: bool = False) -> None:
    """Install all patches. `devices` is the config.yaml device list.

    tolerate_pty_ctl: simulation only - ignore ENOTTY from DTR/RTS on pseudo-terminals.
    """
    global _session_dir, _tolerate_pty_ctl
    _session_dir = session_dir
    _tolerate_pty_ctl = tolerate_pty_ctl
    os.makedirs(session_dir, exist_ok=True)
    for d in devices:
        path = d.get("port") or d.get("device")
        if path:
            _path_to_dev[str(path)] = d["id"]
            _path_to_dev[os.path.realpath(str(path))] = d["id"]
    _patch_serial()
    _patch_usbtmc()


def close_all() -> None:
    with _lock:
        for fh in _files.values():
            fh.close()
        _files.clear()
