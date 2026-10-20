#!/usr/bin/env python3
"""Raw SCPI probe of an OWON DGE over the Linux usbtmc driver (bench tool).

usage: scripts/dge_probe.py /dev/usbtmcN [command ...]   (a command ending in '?' is read back)
Commands with no '?' are sent without reading (the DGE never answers setters). Each line of
output is `command -> reply`. Stop BenchMesh first: one owner per port.
"""
import os, sys, time

def main():
    dev, cmds = sys.argv[1], sys.argv[2:]
    fd = os.open(dev, os.O_RDWR)
    try:
        for c in cmds:
            os.write(fd, (c + "\r").encode())
            if c.rstrip().endswith("?"):
                try:
                    r = os.read(fd, 4096).decode(errors="replace").strip()
                except OSError as e:
                    r = f"<{e}>"
                print(f"{c} -> {r!r}", flush=True)
            else:
                print(f"{c} (sent)", flush=True)
            time.sleep(0.06)
    finally:
        os.close(fd)

if __name__ == "__main__":
    main()
