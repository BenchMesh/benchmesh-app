#!/usr/bin/env python3
"""Bench check of the AWG modes through a running BenchMesh server (no output is switched on).

usage: scripts/awg_modes_check.py BASE_URL INSTRUMENT_ID [CH]
Sets each mode and modulation parameter, waits for the registry to report it, and restores
continuous output (CW) at the end. Prints one line per check.
"""
import asyncio, json, sys, time, urllib.request
import websockets

BASE, DEV = sys.argv[1].rstrip('/'), sys.argv[2]
CH = sys.argv[3] if len(sys.argv) > 3 else '1'

def post(op, v):
    req = urllib.request.Request(f"{BASE}/instruments/AWG/{DEV}/{CH}/{op}/{v}", method='POST')
    try:
        urllib.request.urlopen(req).read()
        return None
    except urllib.error.HTTPError as e:
        return f"{e.code} {e.read().decode()}"

async def status(ws):
    msg = json.loads(await ws.recv())
    return msg.get(DEV, {}).get('AWG', {}).get(f'status_ch{CH}', {})

def val(v):
    return float(v['val']) if isinstance(v, dict) and 'val' in v else v

async def expect(ws, key, want, timeout=15):
    t0 = time.time()
    while time.time() - t0 < timeout:
        st = await status(ws)
        got = val(st.get(key))
        if got == want or (isinstance(want, float) and isinstance(got, (int, float)) and abs(got - want) <= max(abs(want) * 1e-4, 1e-9)):
            return f"ok   {key} = {got!r} after {time.time() - t0:.1f} s"
    return f"FAIL {key} = {got!r}, wanted {want!r}"

async def main():
    url = BASE.replace('http', 'ws') + '/ws/registry'
    async with websockets.connect(url, ping_interval=None, max_queue=None) as ws:
        steps = [
            ('mode', 'SWEEP', 'MODE', 'SWEEP'),
            ('sweep_spacing', 'LOG', 'SWEEP_SPACING', 'LOG'),
            ('sweep_spacing', 'LIN', 'SWEEP_SPACING', 'LIN'),
            ('sweep_time', '2', 'SWEEP_TIME', 2.0),
            ('sweep_time', '1', 'SWEEP_TIME', 1.0),
            ('mode', 'MOD', 'MOD_TYPE', 'AM'),
            ('am_depth', '55', 'AM_DEPTH', 55.0),
            ('am_depth', '100', 'AM_DEPTH', 100.0),
            ('mod_type', 'FM', 'MOD_TYPE', 'FM'),
            ('mod_frequency', '250', 'MOD_FREQ', 250.0),
            ('mod_frequency', '100', 'MOD_FREQ', 100.0),
            ('mod_shape', 'SQU', 'MOD_SHAPE', 'SQU'),
            ('mod_shape', 'SIN', 'MOD_SHAPE', 'SIN'),
            ('mod_type', 'PM', 'MOD_TYPE', 'PM'),
            ('pm_deviation', '90', 'PM_DEV', 90.0),
            ('pm_deviation', '0', 'PM_DEV', 0.0),
            ('mode', 'BURST', 'MODE', 'BURST'),
            ('burst_mode', 'NCYC', 'BURST_MODE', 'NCYC'),
            ('burst_cycles', '5', 'BURST_CYCLES', 5),
            ('burst_mode', 'INF', 'BURST_MODE', 'INF'),
            ('load', '50', 'LOAD', '50'),
            ('load', 'HIGHZ', 'LOAD', 'HIGHZ'),
            ('mode', 'CW', 'MODE', 'CW'),
        ]
        for op, v, key, want in steps:
            err = post(op, v)
            print(f"{op} {v}: " + (f"REFUSED {err}" if err else await expect(ws, key, want)), flush=True)

asyncio.run(main())
