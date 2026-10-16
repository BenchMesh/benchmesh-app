#!/usr/bin/env python3
"""
Exercise a running traced service so the wire capture covers every behaviour.

Phases:
  1. passive  — record /ws/registry snapshots while only background polling runs
                (pairs raw poll bytes with the parsed values the UI receives)
  2. queries  — call every GET (query_*) method of every device/class/channel
  3. setters  — ONLY from an explicit scenario file (--setters), never automatic,
                because setters can energise outputs

Results go to <session>/api.jsonl (request, status, body, timing). The tracer
writes the wire bytes; the two are correlated by timestamp and driver method.

Usage:
    python3 scripts/wire_capture/exercise.py [--session DIR] [--passive 20] [--rounds 3]
    python3 scripts/wire_capture/exercise.py --setters scripts/wire_capture/setters.example.yaml

Setter scenario format (YAML list):
    - {device: tenmapsu-1, class: PSU, channel: 1, method: voltage, value: "5.00",
       then_query: [voltage]}
"""

import argparse
import json
import os
import time
import urllib.error
import urllib.parse
import urllib.request

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DEFAULT_EXCLUDE = ["screenshot"]  # GET dispatch passes channel as first arg -> unsafe in Python impl


class Recorder:
    def __init__(self, base: str, session: str):
        self.base = base.rstrip("/")
        self.fh = open(os.path.join(session, "api.jsonl"), "a", buffering=1)

    def log(self, **rec):
        rec.setdefault("t", time.time())
        self.fh.write(json.dumps(rec, default=str) + "\n")

    def call(self, verb: str, path: str, phase: str, timeout: float = 30.0):
        url = self.base + path
        t = time.time()
        req = urllib.request.Request(url, method=verb)
        try:
            with urllib.request.urlopen(req, timeout=timeout) as r:
                status, raw = r.status, r.read()
        except urllib.error.HTTPError as e:
            status, raw = e.code, e.read()
        except Exception as e:
            status, raw = None, repr(e).encode()
        dur = round((time.time() - t) * 1000, 1)
        try:
            body = json.loads(raw)
        except Exception:
            body = raw.decode("utf-8", "replace")[:2000]
        self.log(t=t, phase=phase, verb=verb, path=path, status=status, dur_ms=dur, body=body)
        return status, body


def wait_ready(rec: Recorder, timeout: float = 30.0):
    end = time.time() + timeout
    while time.time() < end:
        try:
            with urllib.request.urlopen(rec.base + "/status", timeout=2) as r:
                if r.status == 200:
                    return
        except Exception:
            pass
        time.sleep(0.5)
    raise SystemExit(f"service at {rec.base} not ready after {timeout}s")


def passive(rec: Recorder, seconds: float):
    """Snapshot /ws/registry for `seconds` (falls back to GET /instruments polling)."""
    ws_url = rec.base.replace("http", "ws", 1) + "/ws/registry"
    end = time.time() + seconds
    try:
        from websockets.sync.client import connect

        with connect(ws_url, open_timeout=5) as ws:
            last = None
            while time.time() < end:
                try:
                    msg = ws.recv(timeout=1.0)
                except TimeoutError:
                    continue
                if msg != last:  # only log changes
                    rec.log(phase="passive", ws="/ws/registry", body=json.loads(msg))
                    last = msg
        return
    except ImportError:
        pass
    while time.time() < end:
        rec.call("GET", "/instruments", "passive")
        time.sleep(1.0)


def instruments(rec: Recorder):
    status, body = rec.call("GET", "/instruments", "discover")
    if isinstance(body, dict):
        body = body.get("instruments", body.get("items", []))
    return body if isinstance(body, list) else []


def queries(rec: Recorder, rounds: int, exclude: list, only: list):
    for inst in instruments(rec):
        dev = inst.get("id")
        if only and dev not in only:
            continue
        for cls in inst.get("classes", []) or []:
            klass = cls["class"]
            _, mbody = rec.call("GET", f"/instruments/{klass}/{dev}/methods", "discover")
            methods = mbody.get("methods", []) if isinstance(mbody, dict) else []
            gets = [m["name"] for m in methods if m.get("http_method") == "GET"
                    and not any(x in m["name"] for x in exclude)]
            channels = [int(p.rstrip("/").rsplit("/", 1)[-1]) for p in cls.get("channels", ["/1"])]
            print(f"[exercise] {dev} {klass} channels={channels} queries={gets}", flush=True)
            for _ in range(rounds):
                for ch in channels:
                    for name in gets:
                        rec.call("GET", f"/instruments/{klass}/{dev}/{ch}/{name}", "query")


def setters(rec: Recorder, path: str):
    import yaml

    with open(path) as f:
        steps = yaml.safe_load(f) or []
    for s in steps:
        val = urllib.parse.quote(str(s["value"]), safe="")
        base = f"/instruments/{s['class']}/{s['device']}/{s.get('channel', 1)}"
        rec.call("POST", f"{base}/{s['method']}/{val}", "setter")
        for q in s.get("then_query", []) or []:
            rec.call("GET", f"{base}/{q}", "setter_verify")
        time.sleep(float(s.get("pause", 0.5)))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--session", help="capture session dir (default: captures/LATEST)")
    ap.add_argument("--base", help="API base URL (default: from session.json)")
    ap.add_argument("--passive", type=float, default=20.0, help="seconds of passive polling capture")
    ap.add_argument("--rounds", type=int, default=3, help="repetitions of the full query sweep")
    ap.add_argument("--exclude", nargs="*", default=DEFAULT_EXCLUDE)
    ap.add_argument("--only", nargs="*", default=[], help="restrict to these device ids")
    ap.add_argument("--setters", help="YAML setter scenario (explicit, opt-in)")
    args = ap.parse_args()

    session = args.session
    if not session:
        with open(os.path.join(REPO, "captures", "LATEST")) as f:
            session = f.read().strip()
    with open(os.path.join(session, "session.json")) as f:
        meta = json.load(f)
    rec = Recorder(args.base or meta["api"], session)

    wait_ready(rec)
    print(f"[exercise] session {session}", flush=True)
    if args.passive > 0:
        print(f"[exercise] passive capture {args.passive}s", flush=True)
        passive(rec, args.passive)
    if args.rounds > 0:
        queries(rec, args.rounds, args.exclude, args.only)
    if args.setters:
        setters(rec, args.setters)
    print("[exercise] done", flush=True)


if __name__ == "__main__":
    main()
