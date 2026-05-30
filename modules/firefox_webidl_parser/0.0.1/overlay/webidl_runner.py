#!/usr/bin/env python3
"""WebIDL runner — Mozilla `WebIDL.py` wrapped to satisfy rules_web's
`webidl_toolchain_type` runner contract.

CLI contract (from `@rules_web//web/webidl:toolchain.bzl`):

    webidl_runner <out.json> <in1.webidl> [in2.webidl ...]

Parses every input file with Mozilla's WebIDL.py parser, runs the
parser's `finish()` to materialise the interface graph, and emits a
JSON list to `out.json`. Each entry has a `kind` (the IDLObject
subclass name — `IDLInterface`, `IDLDictionary`, `IDLEnum`, etc.)
and an `identifier` (the interface/dictionary name, or `None` for
top-level extended-attribute objects).

v0 emit is deliberately minimal — `kind` + `identifier` only. Future
work: walk each IDLInterface's `members` and emit structured field
records that the polyglot AST HTML emitter can drive directly.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from WebIDL import Parser

# Standard web-globals preamble. Real Mozilla source registers these via
# the Bindings.conf glue + the actual Window.webidl etc. files; for a
# stand-alone runner that's invoked on user-selected .webidl files we
# inject minimal stubs so `[Exposed=Window]` validates. If the user's
# input includes the real Window.webidl this will collide — acceptable
# for v0; if it becomes an issue we can detect-and-skip.
_GLOBALS_PREAMBLE = """
[Global=Window, Exposed=Window]
interface Window {};
[Global=(Worker, DedicatedWorker), Exposed=DedicatedWorker]
interface DedicatedWorkerGlobalScope {};
[Global=(Worker, SharedWorker), Exposed=SharedWorker]
interface SharedWorkerGlobalScope {};
[Global=(Worker, ServiceWorker), Exposed=ServiceWorker]
interface ServiceWorkerGlobalScope {};
[Global=(Worklet, AudioWorklet), Exposed=AudioWorklet]
interface AudioWorkletGlobalScope {};
"""


def _identifier_of(node) -> str | None:
    ident = getattr(node, "identifier", None)
    if ident is None:
        return None
    # IDLIdentifier.__str__ returns the name; fall back to repr if not.
    try:
        return str(ident)
    except Exception:
        return repr(ident)


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(
            "usage: webidl_runner <out.json> <in1.webidl> [in2.webidl ...]",
            file=sys.stderr,
        )
        return 2

    out_path = Path(argv[1])
    in_paths = [Path(p) for p in argv[2:]]

    parser = Parser()
    parser.parse(_GLOBALS_PREAMBLE, filename="<globals-preamble>")
    for src in in_paths:
        parser.parse(src.read_text(), filename=str(src))
    productions = parser.finish()

    ast = [
        {
            "kind": type(p).__name__,
            "identifier": _identifier_of(p),
        }
        for p in productions
    ]
    out_path.write_text(json.dumps(ast, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
