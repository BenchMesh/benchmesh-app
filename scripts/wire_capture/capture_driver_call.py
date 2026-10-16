#!/usr/bin/env python3
"""
Capture the wire traffic of one direct driver method call (bypassing the API).

Used for behaviour the HTTP API cannot reach correctly, e.g. the DGE screenshot
(GET dispatch passes the channel number as `save_path`). The driver and transport
are built exactly as the service builds them (SerialManager._make_driver).

Usage:
    python3 scripts/wire_capture/capture_driver_call.py --config captures/bench-config.yaml \
        --id owon-dge2070 --method query_screenshot --kwargs '{"save_path": "/tmp/x.bmp"}'

Writes captures/<timestamp>-<id>-<method>/ with <id>.jsonl and result.json.
"""

import argparse
import datetime
import json
import os
import sys
import time

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(REPO, "benchmesh-serial-service", "src"))
sys.path.insert(0, os.path.dirname(__file__))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--config", required=True)
    ap.add_argument("--id", required=True)
    ap.add_argument("--method", required=True)
    ap.add_argument("--args", default="[]", help="JSON list of positional args")
    ap.add_argument("--kwargs", default="{}", help="JSON dict of keyword args")
    ap.add_argument("--out", default=os.path.join(REPO, "captures"))
    a = ap.parse_args()

    import yaml

    with open(a.config) as f:
        devices = (yaml.safe_load(f) or {}).get("devices", [])
    dev = next((d for d in devices if d["id"] == a.id), None)
    if dev is None:
        sys.exit(f"device {a.id} not in {a.config}")

    session = os.path.join(a.out, f"{datetime.datetime.now():%Y%m%d-%H%M%S}-{a.id}-{a.method}")
    import tracer

    tracer.install(session, [dev])
    from benchmesh_service.serial_manager import SerialManager

    mgr = SerialManager([dev])
    drv = mgr._make_driver(dev)
    t = time.time()
    try:
        result = getattr(drv, a.method)(*json.loads(a.args), **json.loads(a.kwargs))
        outcome = {"ok": True}
    except Exception as e:
        result, outcome = None, {"ok": False, "error": repr(e)}
    finally:
        drv.close()
        tracer.close_all()
    if isinstance(result, (bytes, bytearray)):
        summary = {"type": "bytes", "len": len(result), "head_hex": bytes(result[:64]).hex()}
    else:
        summary = {"type": type(result).__name__, "value": result}
    with open(os.path.join(session, "result.json"), "w") as f:
        json.dump({"device": dev, "method": a.method, "args": json.loads(a.args), "kwargs": json.loads(a.kwargs),
                   "started": t, "duration_s": round(time.time() - t, 3), **outcome, "result": summary},
                  f, indent=2, default=str)
    print(f"[capture_driver_call] {session}: {outcome} {summary if outcome['ok'] else ''}")


if __name__ == "__main__":
    main()
