#!/usr/bin/env python3
"""
Minimal TENMA 72-series simulator on a pty, used to self-test the wire capture
tooling without hardware. Mimics the real protocol shape: no terminators in
either direction, commands delimited by an idle gap, setters produce no reply,
STATUS? replies with a single binary byte.

Usage: sim_tenma.py /path/to/pty
"""

import os
import re
import select
import sys

state = {"vset": 12.0, "iset": 1.0, "out": True}


def reply(cmd: str):
    if cmd == "*IDN?":
        return b"TENMA 72-2540 V2.1"
    m = re.fullmatch(r"(VOUT|IOUT|VSET|ISET)(\d)\?", cmd)
    if m:
        kind = m.group(1)
        if kind in ("VOUT", "VSET"):
            return f"{state['vset'] if state['out'] or kind == 'VSET' else 0:05.2f}".encode()
        return f"{state['iset'] * 0.5 if kind == 'IOUT' else state['iset']:.3f}".encode()
    if cmd == "STATUS?":
        return bytes([0x51 if state["out"] else 0x11])  # out|beep|ch1 CV
    m = re.fullmatch(r"VSET\d:([\d.]+)", cmd)
    if m:
        state["vset"] = float(m.group(1))
        return None
    m = re.fullmatch(r"ISET\d:([\d.]+)", cmd)
    if m:
        state["iset"] = float(m.group(1))
        return None
    m = re.fullmatch(r"OUT([01])", cmd)
    if m:
        state["out"] = m.group(1) == "1"
        return None
    return None  # unknown commands are silently ignored, like the real unit


def main():
    fd = os.open(sys.argv[1], os.O_RDWR | os.O_NOCTTY)
    buf = b""
    while True:
        r, _, _ = select.select([fd], [], [], 0.02)  # 20 ms idle gap ends a command
        if r:
            buf += os.read(fd, 256)
            continue
        if buf:
            out = reply(buf.decode("ascii", "replace").strip())
            buf = b""
            if out:
                os.write(fd, out)


if __name__ == "__main__":
    main()
