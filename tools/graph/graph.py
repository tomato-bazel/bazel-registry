#!/usr/bin/env python3
"""Consolidated constellation graph + consistency report.

Parses every `MODULE.bazel` under a repos root (one git checkout per dir), builds
the cross-module dependency graph, and emits:

  <out>/graph.dot          - the internal module DAG (render with graphviz `dot`)
  <out>/report.md          - load-bearing modules, version drift, convention gaps
  <out>/ecosystem.ttl      - RDF projection (feed rules_jena SPARQL / SHACL)

Pure stdlib; no Bazel needed. Encapsulates what was previously done by hand so a
scheduled CI job (the fastverk App-driven `report.yml`) can regenerate it.

Usage:
  graph.py --repos-root <dir> --out <dir> [--title "tomato-bazel"]
"""
import argparse, os, glob, re, collections

DEP = re.compile(r'bazel_dep\(\s*name\s*=\s*"([^"]+)"(?:\s*,\s*version\s*=\s*"([^"]+)")?')
MOD = re.compile(r'module\(\s*name\s*=\s*"([^"]+)"\s*,\s*version\s*=\s*"([^"]+)"')
CONV = ["README", "LICENSE", "CHANGELOG", "docs/", "examples/", "tests/", ".bazelrc", "defs.bzl"]


def scan(repos_root):
    data = {}
    for d in sorted(glob.glob(os.path.join(repos_root, "*"))):
        mb = os.path.join(d, "MODULE.bazel")
        if not os.path.isfile(mb):
            continue
        txt = open(mb, encoding="utf-8", errors="ignore").read()
        m = MOD.search(txt)
        name = m.group(1) if m else os.path.basename(d)
        has = lambda p: os.path.exists(os.path.join(d, p))
        isdir = lambda p: os.path.isdir(os.path.join(d, p))
        conv = {
            "README": has("README.md"), "LICENSE": any(has(x) for x in ("LICENSE", "LICENSE.md", "LICENSE.txt")),
            "CHANGELOG": has("CHANGELOG.md"), "docs/": isdir("docs"),
            "examples/": isdir("examples") or isdir("example"), "tests/": isdir("tests") or isdir("test"),
            ".bazelrc": has(".bazelrc"), "defs.bzl": has("defs.bzl") or bool(glob.glob(os.path.join(d, "*", "defs.bzl"))),
        }
        data[name] = {"version": m.group(2) if m else "?", "deps": DEP.findall(txt), "conv": conv}
    return data


def analyze(data):
    mods = set(data)
    indeg = collections.Counter()
    extver = collections.defaultdict(lambda: collections.defaultdict(list))
    edges = []
    for mod, info in data.items():
        for dn, dv in info["deps"]:
            if dn in mods:
                edges.append((mod, dn)); indeg[dn] += 1
            else:
                extver[dn][dv or "?"].append(mod)
    drift = {d: dict(v) for d, v in extver.items() if len([x for x in v if x != "?"]) > 1}
    return indeg, drift, edges


def write_report(data, indeg, drift, out, title):
    miss = collections.Counter()
    for info in data.values():
        for k in CONV:
            if not info["conv"][k]:
                miss[k] += 1
    o = [f"# {title} — consolidated graph & consistency report", "",
         f"- modules: **{len(data)}**  ·  drift deps: **{len(drift)}**", "",
         "## Load-bearing (internal in-degree)"]
    o += [f"- **{n}** ← {c}" for n, c in indeg.most_common(12)]
    o += ["", "## Version drift (one dep, multiple pins)"]
    o += [f"- **{d}**: " + " | ".join(f"`{v}`×{len(m)}" for v, m in sorted(vs.items()))
          for d, vs in sorted(drift.items(), key=lambda kv: -len(kv[1]))] or ["- none"]
    o += ["", f"## Convention gaps (missing, of {len(data)})"]
    o += [f"- {k}: **{miss[k]}**" for k in CONV]
    open(os.path.join(out, "report.md"), "w").write("\n".join(o) + "\n")


def write_dot(data, edges, out):
    dot = ['digraph g {', 'rankdir=LR; bgcolor="#15161A"; pad=0.4;',
           'node[shape=box,style="filled,rounded",fillcolor="#E64A33",color="#3FA34D",fontcolor="#15161A",fontname="Helvetica-Bold",fontsize=11];',
           'edge[color="#9A9488",arrowsize=0.7];']
    dot += [f'  "{m}";' for m in data] + [f'  "{a}" -> "{b}";' for a, b in edges] + ["}"]
    open(os.path.join(out, "graph.dot"), "w").write("\n".join(dot) + "\n")


def write_ttl(data, out):
    t = ['@prefix fv: <https://tomato-bazel.dev/ns#> .', '']
    for mod, info in data.items():
        s = f'fv:{mod.replace("-", "_")}'
        cls = "fv:Module, fv:RulesModule" if mod.startswith("rules_") else "fv:Module"
        t.append(f'{s} a {cls} ; fv:version "{info["version"]}" ;')
        for k in CONV:
            t.append(f'    fv:has_{k.strip("/.").replace(".", "")} {str(info["conv"][k]).lower()} ;')
        t[-1] = t[-1][:-1] + " ."
    open(os.path.join(out, "ecosystem.ttl"), "w").write("\n".join(t) + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repos-root", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--title", default="tomato-bazel")
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    data = scan(a.repos_root)
    indeg, drift, edges = analyze(data)
    write_report(data, indeg, drift, a.out, a.title)
    write_dot(data, edges, a.out)
    write_ttl(data, a.out)
    print(f"modules={len(data)} edges={len(edges)} drift={len(drift)} -> {a.out}")


if __name__ == "__main__":
    main()
