#!/usr/bin/env python3
"""
Run the existing Python BenchMesh service with the wire tracer installed.

Creates a capture session directory (captures/<timestamp>/) holding per-device
JSONL wire logs plus session.json (config, manifests, environment).

Usage (from repo root):
    python3 scripts/wire_capture/run_traced_service.py [--config ~/.benchmesh/config.yaml]

Stop with Ctrl-C (SIGINT/SIGTERM): the service shuts down and closes ports.
"""

import argparse
import datetime
import json
import os
import shutil
import subprocess
import sys

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC = os.path.join(REPO, "benchmesh-serial-service", "src")
DRIVERS = os.path.join(SRC, "benchmesh_service", "drivers")
sys.path.insert(0, SRC)
sys.path.insert(0, os.path.dirname(__file__))


def processes_holding(path: str) -> list:
    """PIDs (other than ours) that have `path` open - instruments must not be shared."""
    real = os.path.realpath(path)
    pids = []
    for pid in filter(str.isdigit, os.listdir("/proc")):
        if int(pid) == os.getpid():
            continue
        fd_dir = f"/proc/{pid}/fd"
        try:
            for fd in os.listdir(fd_dir):
                if os.path.realpath(os.path.join(fd_dir, fd)) == real:
                    pids.append(int(pid))
                    break
        except OSError:
            continue
    return pids


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--config", default=os.path.expanduser("~/.benchmesh/config.yaml"))
    ap.add_argument("--out", default=os.path.join(REPO, "captures"))
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=57990, help="API port (default 57990, avoids the normal 57666)")
    ap.add_argument("--legacy-polling", action="store_true", help="disable unified polling (start.sh enables it)")
    ap.add_argument("--force", action="store_true", help="start even if another process holds an instrument port")
    ap.add_argument("--sim-pty", action="store_true", help="simulation only: tolerate DTR/RTS ioctl failure on ptys")
    args = ap.parse_args()

    import yaml

    with open(args.config) as f:
        cfg = yaml.safe_load(f) or {}
    devices = cfg.get("devices", []) or []

    busy = {}
    for d in devices:
        path = d.get("port") or d.get("device")
        if path and os.path.exists(path):
            pids = processes_holding(path)
            if pids:
                busy[path] = pids
    if busy and not args.force:
        sys.exit(f"Instrument ports are in use by other processes {busy}. Stop the normal service first (or --force).")

    session = os.path.join(args.out, datetime.datetime.now().strftime("%Y%m%d-%H%M%S"))
    os.makedirs(session, exist_ok=True)
    data_dir = os.path.join(session, "_datadir")  # keeps the user's recordings.db untouched
    os.makedirs(data_dir, exist_ok=True)

    os.environ["BENCHMESH_CONFIG"] = os.path.abspath(args.config)
    os.environ["BENCHMESH_DATA_DIR"] = data_dir
    os.environ["BENCHMESH_START_UI"] = "0"
    os.environ["BM_UNIFIED_POLLING"] = "false" if args.legacy_polling else "true"

    manifests = {}
    for d in devices:
        name = d.get("driver", "")
        mpath = os.path.join(DRIVERS, name, "manifest.json")
        if os.path.exists(mpath):
            with open(mpath) as f:
                manifests[name] = json.load(f)
    try:
        git_rev = subprocess.check_output(["git", "-C", REPO, "rev-parse", "HEAD"], text=True).strip()
    except Exception:
        git_rev = None
    meta = {
        "started": datetime.datetime.now().isoformat(),
        "git_rev": git_rev,
        "python": sys.version,
        "config_path": os.path.abspath(args.config),
        "config": cfg,
        "manifests": manifests,
        "env": {k: v for k, v in os.environ.items() if k.startswith(("BM_", "BENCHMESH_"))},
        "api": f"http://{args.host}:{args.port}",
        "simulated": args.sim_pty,
    }
    with open(os.path.join(session, "session.json"), "w") as f:
        json.dump(meta, f, indent=2, default=str)
    with open(os.path.join(args.out, "LATEST"), "w") as f:
        f.write(session + "\n")

    import tracer

    tracer.install(session, devices, tolerate_pty_ctl=args.sim_pty)
    print(f"[wire_capture] session: {session}", flush=True)
    print(f"[wire_capture] API:     http://{args.host}:{args.port}", flush=True)

    import uvicorn

    try:
        uvicorn.run("benchmesh_service.api:app", host=args.host, port=args.port, log_level="warning")
    finally:
        tracer.close_all()
        shutil.rmtree(data_dir, ignore_errors=True)
        print(f"[wire_capture] closed. summarize with: python3 scripts/wire_capture/summarize.py {session}", flush=True)


if __name__ == "__main__":
    main()
