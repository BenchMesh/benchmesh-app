#!/usr/bin/env python3
"""Extract protocol test vectors from open-source instrument libraries into neutral JSON.

Dev tooling only (never shipped). The libraries are parsed with `ast` / YAML; nothing of
theirs is imported or executed. Output goes OUTSIDE the repository (licences: InstrumentKit is
AGPLv3, so its tests are used as facts only and never copied into BenchMesh).

Sources:
  pymeasure     tests/**/test_*.py   expected_protocol(cls, [(write, reply), ...])
  instrumentkit tests/**/test_*.py   expected_protocol(cls, [writes], [replies], sep="\\n")
  qcodes        src/**/sims/*.yaml   pyvisa-sim dialogues and properties

Output: <out>/<library>/<instrument_class>.json, a list of
  {library, file, test, instrument_class, exchanges: [{write, reply|null}], note?}

Usage: python extract.py [--libs DIR] [--out DIR] [--only pymeasure,qcodes,instrumentkit]
"""
from __future__ import annotations

import argparse
import ast
import itertools
import json
import sys
from collections import defaultdict
from pathlib import Path

import yaml

DOC = Path(__file__).resolve().parents[3] / "BenchMeshDOC"
DEFAULT_LIBS = DOC / "_libs"
DEFAULT_OUT = DOC / "_vectors"


class Unresolved(Exception):
    """An expression the mini evaluator cannot (or must not) evaluate."""


# --------------------------------------------------------------------------- evaluator

_BINOPS = {
    ast.Add: lambda a, b: a + b,
    ast.Sub: lambda a, b: a - b,
    ast.Mult: lambda a, b: a * b,
    ast.Div: lambda a, b: a / b,
    ast.Mod: lambda a, b: a % b,
    ast.Pow: lambda a, b: a ** b,
    ast.FloorDiv: lambda a, b: a // b,
}
_SAFE_CALLS = {"str": str, "int": int, "float": float, "range": range, "len": len,
               "repr": repr, "bytes": bytes, "list": list, "tuple": tuple, "round": round,
               "abs": abs, "chr": chr}
_SAFE_METHODS = {"format", "encode", "decode", "upper", "lower", "strip", "rstrip", "lstrip",
                 "replace", "join", "split"}


def ev(node: ast.AST, env: dict):
    """Evaluate a literal-ish expression: constants, containers, names in `env`, f-strings,
    `%` / `.format`, arithmetic, simple comprehensions. Anything else raises Unresolved."""
    if isinstance(node, ast.Constant):
        return node.value
    if isinstance(node, (ast.List, ast.Tuple)):
        out = []
        for e in node.elts:
            if isinstance(e, ast.Starred):
                out.extend(ev(e.value, env))
            else:
                out.append(ev(e, env))
        return out if isinstance(node, ast.List) else tuple(out)
    if isinstance(node, ast.Dict):
        return {ev(k, env): ev(v, env) for k, v in zip(node.keys, node.values)}
    if isinstance(node, ast.Name):
        if node.id in env:
            return env[node.id]
        if node.id in ("True", "False", "None"):
            return {"True": True, "False": False, "None": None}[node.id]
        raise Unresolved(f"name {node.id}")
    if isinstance(node, ast.UnaryOp):
        v = ev(node.operand, env)
        if isinstance(node.op, ast.USub):
            return -v
        if isinstance(node.op, ast.UAdd):
            return +v
        if isinstance(node.op, ast.Not):
            return not v
        raise Unresolved("unary")
    if isinstance(node, ast.BinOp):
        op = _BINOPS.get(type(node.op))
        if op is None:
            raise Unresolved("binop")
        return op(ev(node.left, env), ev(node.right, env))
    if isinstance(node, ast.JoinedStr):
        parts = []
        for v in node.values:
            if isinstance(v, ast.Constant):
                parts.append(v.value)
            elif isinstance(v, ast.FormattedValue):
                x = ev(v.value, env)
                if v.conversion == ord("r"):
                    x = repr(x)
                elif v.conversion == ord("s"):
                    x = str(x)
                spec = ev(v.format_spec, env) if v.format_spec else ""
                parts.append(format(x, spec))
            else:
                raise Unresolved("fstring part")
        return "".join(parts)
    if isinstance(node, ast.Subscript):
        base = ev(node.value, env)
        sl = node.slice
        if isinstance(sl, ast.Slice):
            lo = ev(sl.lower, env) if sl.lower else None
            hi = ev(sl.upper, env) if sl.upper else None
            return base[lo:hi]
        return base[ev(sl, env)]
    if isinstance(node, ast.IfExp):
        return ev(node.body, env) if ev(node.test, env) else ev(node.orelse, env)
    if isinstance(node, ast.Compare) and len(node.ops) == 1:
        a, b = ev(node.left, env), ev(node.comparators[0], env)
        if isinstance(node.ops[0], ast.Eq):
            return a == b
        if isinstance(node.ops[0], ast.NotEq):
            return a != b
        raise Unresolved("compare")
    if isinstance(node, ast.Call):
        args = [ev(a, env) for a in node.args]
        kwargs = {k.arg: ev(k.value, env) for k in node.keywords if k.arg}
        f = node.func
        if isinstance(f, ast.Name) and f.id in _SAFE_CALLS:
            return _SAFE_CALLS[f.id](*args, **kwargs)
        if isinstance(f, ast.Attribute) and f.attr in _SAFE_METHODS:
            obj = ev(f.value, env)
            if isinstance(obj, (str, bytes)):
                return getattr(obj, f.attr)(*args, **kwargs)
        raise Unresolved("call")
    if isinstance(node, ast.ListComp) and len(node.generators) == 1:
        g = node.generators[0]
        out = []
        for item in ev(g.iter, env):
            sub = dict(env)
            bind(g.target, item, sub)
            if all(ev(c, sub) for c in g.ifs):
                out.append(ev(node.elt, sub))
        return out
    raise Unresolved(type(node).__name__)


def bind(target: ast.AST, value, env: dict):
    if isinstance(target, ast.Name):
        env[target.id] = value
    elif isinstance(target, (ast.Tuple, ast.List)):
        vals = list(value)
        if len(vals) != len(target.elts):
            raise Unresolved("unpack")
        for t, v in zip(target.elts, vals):
            bind(t, v, env)
    else:
        raise Unresolved("target")


def text(x) -> str | None:
    """Wire text of a write or reply value (bytes as latin-1, numbers via str())."""
    if x is None:
        return None
    if isinstance(x, bytes):
        return x.decode("latin-1")
    if isinstance(x, bool):
        return str(int(x))
    return str(x)


# --------------------------------------------------------------------------- python tests


def class_name(node: ast.AST) -> str:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        return node.attr
    if isinstance(node, ast.Call):  # e.g. functools.partial(Cls, ...)
        return class_name(node.args[0]) if node.args else "?"
    return "?"


def module_env(tree: ast.Module) -> dict:
    env: dict = {}
    for st in tree.body:
        if isinstance(st, ast.Assign):
            try:
                v = ev(st.value, env)
            except Exception:
                continue
            for t in st.targets:
                try:
                    bind(t, v, env)
                except Exception:
                    pass
    return env


def parametrizations(fn: ast.FunctionDef, env: dict) -> list[tuple[str, dict]]:
    """[(id suffix, {param: value})] over stacked @pytest.mark.parametrize decorators."""
    axes = []
    for d in fn.decorator_list:
        if not (isinstance(d, ast.Call) and isinstance(d.func, ast.Attribute) and d.func.attr == "parametrize"):
            continue
        try:
            names = ev(d.args[0], env)
            values = ev(d.args[1], env)
        except Exception:
            return []  # cannot expand: skip the test rather than guess
        names = [n.strip() for n in names.split(",")] if isinstance(names, str) else list(names)
        rows = []
        for v in values:
            row = v if len(names) > 1 else (v,)
            rows.append(dict(zip(names, row)))
        axes.append(rows)
    if not axes:
        return [("", {})]
    out = []
    for combo in itertools.product(*[list(enumerate(a)) for a in axes]):
        merged = {}
        for _, row in combo:
            merged.update(row)
        out.append(("[" + "-".join(str(i) for i, _ in combo) + "]", merged))
    return out


def is_expected_protocol(node: ast.AST) -> bool:
    return isinstance(node, ast.Call) and class_name(node.func) == "expected_protocol"


def walk_body(stmts, env, on_call):
    """Statements in order: track simple assignments and for-loops; report every
    expected_protocol call with the environment valid at that point."""
    for st in stmts:
        if isinstance(st, ast.Assign):
            try:
                v = ev(st.value, env)
                for t in st.targets:
                    bind(t, v, env)
            except Exception:
                pass
        elif isinstance(st, (ast.With, ast.AsyncWith)):
            for item in st.items:
                if is_expected_protocol(item.context_expr):
                    on_call(item.context_expr, dict(env))
            walk_body(st.body, env, on_call)
        elif isinstance(st, ast.For):
            try:
                items = list(ev(st.iter, env))
            except Exception:
                walk_body(st.body, env, on_call)
                continue
            for it in items:
                sub = dict(env)
                try:
                    bind(st.target, it, sub)
                except Exception:
                    continue
                walk_body(st.body, sub, on_call)
        elif isinstance(st, ast.If):
            walk_body(st.body, env, on_call)
            walk_body(st.orelse, env, on_call)
        elif isinstance(st, ast.Try):
            walk_body(st.body, env, on_call)
        elif isinstance(st, ast.Expr) and is_expected_protocol(st.value):
            on_call(st.value, dict(env))


def pymeasure_exchanges(call: ast.Call, env: dict):
    pairs = ev(call.args[1], env) if len(call.args) > 1 else ev(next(k.value for k in call.keywords if k.arg == "comm_pairs"), env)
    out = []
    for p in pairs:
        w, r = p
        out.append({"write": text(w), "reply": text(r)})
    return out, None


def ik_exchanges(call: ast.Call, env: dict):
    kw = {k.arg: k.value for k in call.keywords}
    host = ev(call.args[1] if len(call.args) > 1 else kw["host_to_ins"], env)
    ins = ev(call.args[2] if len(call.args) > 2 else kw["ins_to_host"], env)
    sep = ev(call.args[3], env) if len(call.args) > 3 else (ev(kw["sep"], env) if "sep" in kw else "\n")
    repeat = ev(kw["repeat"], env) if "repeat" in kw else 1
    sep = text(sep)

    def lines(v):
        if isinstance(v, (str, bytes)):
            v = [x for x in text(v).split(sep)]
            return [x for x in v if x != ""]
        return [text(x) for x in v]

    writes, replies = lines(host) * repeat, lines(ins) * repeat
    # replies belong to queries, in order (InstrumentKit lists them separately)
    queries = [i for i, w in enumerate(writes) if "?" in w]
    note = None
    if len(queries) != len(replies):
        note = f"pairing uncertain: {len(queries)} queries, {len(replies)} replies"
    out = [{"write": w, "reply": None} for w in writes]
    for qi, r in zip(queries, replies):
        out[qi]["reply"] = r
    return out, note


def extract_python(library: str, root: Path, tests_dir: Path, exchanges_of) -> tuple[dict, dict]:
    vectors: dict[str, list] = defaultdict(list)
    stats = {"files": 0, "calls": 0, "unresolved": 0}
    for f in sorted(tests_dir.rglob("test_*.py")):
        src = f.read_text(encoding="utf-8", errors="replace")
        if "expected_protocol" not in src:
            continue
        try:
            tree = ast.parse(src)
        except SyntaxError:
            continue
        stats["files"] += 1
        menv = module_env(tree)
        rel = str(f.relative_to(root))

        def functions():
            for st in tree.body:
                if isinstance(st, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    yield "", st
                elif isinstance(st, ast.ClassDef):
                    for m in st.body:
                        if isinstance(m, (ast.FunctionDef, ast.AsyncFunctionDef)):
                            yield st.name + "::", m

        for prefix, fn in functions():
            if not fn.name.startswith("test"):
                continue
            for suffix, params in parametrizations(fn, menv):
                env = dict(menv)
                env.update(params)
                calls = []
                walk_body(fn.body, env, lambda c, e: calls.append((c, e)))
                for n, (call, cenv) in enumerate(calls):
                    stats["calls"] += 1
                    try:
                        exchanges, note = exchanges_of(call, cenv)
                    except Exception as e:  # noqa: BLE001 - anything unevaluable is skipped
                        stats["unresolved"] += 1
                        continue
                    cls = class_name(call.args[0])
                    rec = {"library": library, "file": rel, "test": f"{prefix}{fn.name}{suffix}" + (f"#{n}" if len(calls) > 1 else ""),
                           "instrument_class": cls, "exchanges": exchanges}
                    if note:
                        rec["note"] = note
                    vectors[cls].append(rec)
    return vectors, stats


# --------------------------------------------------------------------------- qcodes sims


def sim_reply(template: str | None, default):
    """pyvisa-sim getter reply: a str.format template ("{}", "{:.15E}") over the default."""
    if template is None or default is None:
        return template
    for v in (default, _number(default)):
        try:
            return template.format(v)
        except (ValueError, TypeError, IndexError, KeyError):
            continue
    return template


def _number(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return x


def extract_qcodes(root: Path) -> tuple[dict, dict]:
    vectors: dict[str, list] = defaultdict(list)
    stats = {"files": 0, "dialogues": 0, "properties": 0}
    for f in sorted(root.rglob("sims/*.yaml")):
        try:
            doc = yaml.safe_load(f.read_text(encoding="utf-8"))
        except yaml.YAMLError:
            continue
        if not isinstance(doc, dict) or "devices" not in doc:
            continue
        stats["files"] += 1
        cls = f.stem
        rel = str(f.relative_to(root))
        for dev_name, dev in (doc.get("devices") or {}).items():
            if not isinstance(dev, dict):
                continue
            ex = []
            for d in dev.get("dialogues") or []:
                r = d.get("r")
                ex.append({"write": text(d.get("q")), "reply": None if r in (None, "null_response") else text(r)})
                stats["dialogues"] += 1
            if ex:
                vectors[cls].append({"library": "qcodes", "file": rel, "test": f"{dev_name}/dialogues", "instrument_class": cls,
                                     "exchanges": ex, "note": "pyvisa-sim replies are simulator values, not instrument captures"})
            for pname, p in (dev.get("properties") or {}).items():
                if not isinstance(p, dict):
                    continue
                stats["properties"] += 1
                default = text(p.get("default"))
                ex = []
                g, s = p.get("getter"), p.get("setter")
                if isinstance(s, dict) and s.get("q"):
                    q = text(s["q"])
                    ex.append({"write": q.replace("{}", default) if default is not None else q, "reply": None})
                if isinstance(g, dict) and g.get("q"):
                    ex.append({"write": text(g["q"]), "reply": sim_reply(text(g.get("r")), p.get("default"))})
                if ex:
                    vectors[cls].append({"library": "qcodes", "file": rel, "test": f"{dev_name}/{pname}", "instrument_class": cls,
                                         "exchanges": ex, "note": "pyvisa-sim property: reply is the simulator default"})
    return vectors, stats


# --------------------------------------------------------------------------- main


def write(out: Path, library: str, vectors: dict):
    d = out / library
    d.mkdir(parents=True, exist_ok=True)
    for cls, recs in vectors.items():
        safe = "".join(c if c.isalnum() or c in "_-." else "_" for c in cls)
        (d / f"{safe}.json").write_text(json.dumps(recs, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--libs", type=Path, default=DEFAULT_LIBS)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--only", default="pymeasure,instrumentkit,qcodes")
    a = ap.parse_args(argv)
    repo = Path(__file__).resolve().parents[2]
    if a.out.resolve().is_relative_to(repo):
        print("refusing to write vectors inside the BenchMesh repository (licences)", file=sys.stderr)
        return 2
    only = set(a.only.split(","))
    if "pymeasure" in only:
        root = a.libs / "pymeasure"
        v, s = extract_python("pymeasure", root, root / "tests", pymeasure_exchanges)
        write(a.out, "pymeasure", v)
        print(f"pymeasure: {len(v)} classes, {sum(map(len, v.values()))} tests, {s}")
    if "instrumentkit" in only:
        root = a.libs / "InstrumentKit"
        v, s = extract_python("instrumentkit", root, root / "tests", ik_exchanges)
        write(a.out, "instrumentkit", v)
        print(f"instrumentkit: {len(v)} classes, {sum(map(len, v.values()))} tests, {s}")
    if "qcodes" in only:
        root = a.libs / "Qcodes"
        v, s = extract_qcodes(root)
        write(a.out, "qcodes", v)
        print(f"qcodes: {len(v)} classes, {sum(map(len, v.values()))} tests, {s}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
