#!/usr/bin/env python3
"""
Turn raw wire-capture JSONL into human-readable transcripts and per-command stats.

For each <device>.jsonl in a session it writes:
  <device>.transcript.txt   one line per exchange (TX + coalesced RX)
  <device>.stats.json       per-command reply variants, sizes, latency, end reason, gaps

An exchange is a TX followed by every RX until the next TX. Timing note: the
tracer stamps each event when the underlying call returns. "latency" is the time
from TX completion until the first read that returned data (for timeout-framed
devices this includes the read timeout). End reasons:
  timeout   — the last read returned fewer bytes than requested (reply framed by timeout)
  complete  — the last read was satisfied (terminator / requested size reached)
  no_reply  — no bytes received before the next TX

Usage:
    python3 scripts/wire_capture/summarize.py [SESSION_DIR]
"""

import json
import os
import statistics
import sys
from collections import Counter, defaultdict

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))


def load(path):
    with open(path) as f:
        return [json.loads(line) for line in f if line.strip()]


def exchanges(events):
    cur = None
    for e in events:
        if e["ev"] == "tx":
            if cur:
                yield cur
            # tracer stamps `mono` when the call returns: start = mono - dur
            cur = {"tx": bytes.fromhex(e["hex"]), "t": e["mono"] - e["dur_ms"] / 1000, "tx_end": e["mono"],
                   "op": e.get("op"), "fn": e.get("fn"), "rx": b"", "first_rx": None, "last_rx": None,
                   "reads": 0, "end": "no_reply", "errors": []}
        elif cur is None:
            continue
        elif e["ev"] == "rx":
            data = bytes.fromhex(e["hex"])
            end_t = e["mono"]
            cur["reads"] += 1
            if data:
                cur["rx"] += data
                if cur["first_rx"] is None:
                    cur["first_rx"] = end_t
                cur["last_rx"] = end_t
            cur["end"] = "timeout" if len(data) < e["req"] else "complete"
            if not cur["rx"]:
                cur["end"] = "no_reply"
            cur["rx_done"] = end_t
        elif e["ev"] == "error":
            cur["errors"].append(e.get("error"))
        elif e["ev"] in ("open", "close"):
            yield cur
            cur = None
    if cur:
        yield cur


def esc(b: bytes) -> str:
    return b.decode("latin-1").encode("unicode_escape").decode("ascii")


def command_key(tx: bytes) -> str:
    """Group commands by verb: strip numeric arguments after ':' or ' '."""
    s = esc(tx)
    for sep in (" ", ":"):
        head, _, tail = s.partition(sep)
        if tail and any(c.isdigit() for c in tail) and not tail.rstrip("?\\r\\n").isalpha():
            return head + sep + "<arg>"
    return s


def summarize_device(path):
    events = load(path)
    base = path[:-len(".jsonl")]
    stats = defaultdict(lambda: {"count": 0, "replies": Counter(), "rx_len": [], "latency_ms": [],
                                 "total_ms": [], "end": Counter(), "ops": Counter(), "gap_before_ms": []})
    lines = []
    prev_done = None
    opens = [e for e in events if e["ev"] in ("open", "ctl")]
    for o in opens:
        lines.append(f"{o['mono']:10.3f}  {o['ev'].upper():5} {json.dumps({k: v for k, v in o.items() if k not in ('t', 'mono', 'ev', 'op', 'fn', 'thread')})}")
    for x in exchanges(events):
        k = command_key(x["tx"])
        s = stats[k]
        s["count"] += 1
        s["replies"][esc(x["rx"])] += 1
        s["rx_len"].append(len(x["rx"]))
        s["end"][x["end"]] += 1
        s["ops"][f"{x['op']}/{x['fn']}"] += 1
        if x["first_rx"] is not None:
            s["latency_ms"].append((x["first_rx"] - x["tx_end"]) * 1000)
        if "rx_done" in x:
            s["total_ms"].append((x["rx_done"] - x["t"]) * 1000)
        if prev_done is not None:
            s["gap_before_ms"].append((x["t"] - prev_done) * 1000)
        prev_done = x.get("rx_done", x["tx_end"])
        total = f"{(x['rx_done'] - x['t']) * 1000:7.1f}ms" if "rx_done" in x else "      -  "
        err = f"  ERR {x['errors']}" if x["errors"] else ""
        lines.append(f"{x['t']:10.3f}  TX {esc(x['tx'])!r:28} RX {esc(x['rx'])!r:40} {len(x['rx']):4}B {total} "
                     f"{x['end']:8} [{x['op']}/{x['fn']}]{err}")

    def agg(v):
        return {"min": round(min(v), 2), "median": round(statistics.median(v), 2), "max": round(max(v), 2)} if v else None

    out = {}
    for k, s in sorted(stats.items()):
        out[k] = {"count": s["count"], "reply_variants": len(s["replies"]),
                  "top_replies": s["replies"].most_common(5), "rx_len": agg(s["rx_len"]),
                  "first_byte_latency_ms": agg(s["latency_ms"]), "exchange_ms": agg(s["total_ms"]),
                  "gap_before_ms": agg(s["gap_before_ms"]), "end": dict(s["end"]), "ops": dict(s["ops"])}
    with open(base + ".transcript.txt", "w") as f:
        f.write("\n".join(lines) + "\n")
    with open(base + ".stats.json", "w") as f:
        json.dump(out, f, indent=2)
    return out


def main():
    session = sys.argv[1] if len(sys.argv) > 1 else open(os.path.join(REPO, "captures", "LATEST")).read().strip()
    for name in sorted(os.listdir(session)):
        if not name.endswith(".jsonl") or name == "api.jsonl":
            continue
        dev = name[:-len(".jsonl")]
        out = summarize_device(os.path.join(session, name))
        print(f"\n=== {dev}")
        print(f"{'command':28} {'n':>5} {'rxB':>9} {'latency ms':>12} {'exchange ms':>12}  end / sample reply")
        for k, s in out.items():
            rx = s["rx_len"]
            lat = s["first_byte_latency_ms"]
            ex = s["exchange_ms"]
            sample = s["top_replies"][0][0] if s["top_replies"] else ""
            print(f"{k[:28]:28} {s['count']:5} {str(rx['min']) + '-' + str(rx['max']) if rx else '-':>9} "
                  f"{lat['median'] if lat else '-':>12} {ex['median'] if ex else '-':>12}  {s['end']} {sample[:40]!r}")
    print(f"\ntranscripts and stats written to {session}")


if __name__ == "__main__":
    main()
