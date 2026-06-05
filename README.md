# fastverk bazel-registry

Bzlmod registry for fastverk Bazel modules.

The registry repo itself is public. It describes where to fetch each
module source tarball. Some module entries point at private GitHub
repos and require auth (covered below).

## Modules

Organized by family. Some entries point at private GitHub repos, so
fetching tarballs may require auth (see "Auth" below).

### Polyglot translation + ML training

<!-- BOTNOC:MODULES_TABLE -->
| Module | Latest | Description |
|---|---|---|
| [`rules_lang`](https://github.com/fastverk/rules_lang) | 0.0.0 | Polyglot AST round-trip + translator engine. Provides `Polyglot.Sql.Ast` (the generic 32-ctor SQL AST that `Pg.Ast` extends via the Pattern-A `Ext` slot). |
| [`rules_lora`](https://github.com/fastverk/rules_lora) | 0.0.1 | Bazel-native LoRA fine-tuning. Four declarative macros: `lora_dataset`, `lora_recipe`, `lora_train`, `expert_manifest`. RunPod / local-CPU backends; torchtune-rendered recipes. |
| [`rules_meson`](https://github.com/fastverk/rules_meson) | 0.0.0 | Hermetic meson + ninja toolchain for Bazel. `meson_configure` runs `meson setup` as a build action; consumers get a deterministic `compile_commands.json`. |

### UI + rendering (proto-driven + WGSL)

| Module | Latest | Description |
|---|---|---|
| [`meridian`](https://github.com/mattmarshall/meridian) | 0.1.0 | Proto-driven multi-platform UI framework. Descriptor-driven renderers across TS/Java/Rust surfaces. |
| [`pinax`](https://github.com/mattmarshall/pinax) | 0.1.0 | Meridian-backed JVM application + adhoc-factory web surface (sparql / graph / knowledge probes). |
| [`rules_naga`](https://github.com/fastverk/rules_naga) | 0.6.1 | Bazel-native WGSL validation, composition, and JS-module emission. Wraps `naga` (Mozilla's WGSL compiler) as a `rust_binary` driver. |
| [`wgsl_stdlib`](https://github.com/fastverk/wgsl_stdlib) | 0.4.0 | Reusable WebGPU shader snippets (colormap, complex math, ζ-function, lighting, mesh, contour/grid) validated via `rules_naga`. |
| [`rules_render`](https://github.com/fastverk/rules_render) | 0.3.0 | Bazel-native WGSL rendering framework. Typed providers + rules for materials, surfaces, scenes, passes, pipelines, and apps. |
| [`rules_walkthrough`](https://github.com/fastverk/rules_walkthrough) | 0.1.0 | Bazel rules for declarative slide-deck rendering: `walkthrough.v1.Walkthrough` JSON → self-contained static site (renderer JS + KaTeX + marked + per-deck data sidecars). |

### Embedded systems (seL4 / microkit / hardware)

| Module | Latest | Description |
|---|---|---|
| [`rules_sel4`](https://github.com/fastverk/rules_sel4) | 0.0.1 | Bazel rules for building the seL4 microkernel from source for multiple architectures and platforms. |
| [`rules_microkit`](https://github.com/fastverk/rules_microkit) | 0.0.1 | Bazel rules for seL4 microkit apps: `microkit_pd` / `microkit_system` / `microkit_image`. |
| [`rules_microkit_tool`](https://github.com/fastverk/rules_microkit_tool) | 0.0.1 | Bazel rules building the seL4 microkit Rust binary as a registerable toolchain. |
| [`rules_cc_cross`](https://github.com/fastverk/rules_cc_cross) | 0.1.0 | Hermetic ARM/RISC-V/x86 cross-compiler toolchains for embedded Bazel builds (seL4, microkit, bare-metal). |
| [`rules_qemu`](https://github.com/fastverk/rules_qemu) | 0.1.0 | Hermetic `qemu-system-*` toolchains + `qemu_run` / `qemu_test` rules for booting embedded images under Bazel. |

### Hardware design (HDL / EDA)

| Module | Latest | Description |
|---|---|---|
| [`rules_chisel`](https://github.com/fastverk/rules_chisel) | 0.0.1 | Bazel rules for Chisel HDL: Mill-driven `chisel_module` → Verilog elaboration. |
| [`rules_verilog`](https://github.com/fastverk/rules_verilog) | 0.0.1 | Bazel rules for Verilog/SystemVerilog: Verilator + Icarus simulation, Yosys synthesis, hermetic toolchains. |
| [`rules_kicad`](https://github.com/fastverk/rules_kicad) | 0.2.0 | Bazel rules for KiCad EDA: schematic / pcb / netlist / gerbers / bom via `kicad-cli`. |
| [`rules_riscv_core`](https://github.com/fastverk/rules_riscv_core) | 0.0.1 | Curated RISC-V soft-core presets (Rocket, Ibex, ...) as Bazel-native `riscv_core` targets. |
| [`rules_board`](https://github.com/fastverk/rules_board) | 0.0.1 | Bazel glue rule binding a KiCad PCB + optional soft-CPU to a microkit platform target. |
<!-- /BOTNOC:MODULES_TABLE -->

## Quick start

Add the registry chain to your consumer's `.bazelrc`:

```
common --registry=https://raw.githubusercontent.com/fastverk/bazel-registry/main/
common --registry=https://bcr.bazel.build/
```

Put this registry before BCR so its entries win for overlapping module
names.

Then declare the dep in `MODULE.bazel`:

```python
bazel_dep(name = "rules_lora", version = "0.0.1")
```

That's it — no `local_path_override` needed. The registry resolves
the module's `MODULE.bazel` + `source.json`; Bazel fetches the tarball
via the source.json URL (GitHub auth may be required — see below).

## Auth

For module entries that point at private GitHub repos, Bazel needs to
forward GitHub credentials when fetching. Two equivalent approaches:

### Option A — Bazel credential helper (recommended)

Project-local, doesn't pollute global state. Drop a small shell
script into your repo and reference it from `.bazelrc`. The canonical
implementation lives in
[`rules_postgres/tools/credhelper/gh-cred-helper.sh`](https://github.com/fastverk/rules_postgres/blob/main/tools/credhelper/gh-cred-helper.sh)
— ~30 LOC, forwards `gh auth token` (or `GITHUB_TOKEN` / `GH_TOKEN`
env) as a Bearer auth header.

Wire it in `.bazelrc`:

```
common --credential_helper=*.github.com=%workspace%/tools/credhelper/gh-cred-helper.sh
common --credential_helper=github.com=%workspace%/tools/credhelper/gh-cred-helper.sh
common --credential_helper=raw.githubusercontent.com=%workspace%/tools/credhelper/gh-cred-helper.sh
common --credential_helper=codeload.github.com=%workspace%/tools/credhelper/gh-cred-helper.sh
```

Then any `bazel build`/`bazel run` invocation just works — the helper
picks up your `gh` CLI auth automatically.

### Option B — `~/.netrc`

Simpler, but writes a long-lived token to a file in your home
directory. Add to `~/.netrc`:

```
machine api.github.com
  login <your-github-username>
  password <gh-personal-access-token-with-repo-scope>

machine codeload.github.com
  login <your-github-username>
  password <gh-personal-access-token-with-repo-scope>

machine raw.githubusercontent.com
  login <your-github-username>
  password <gh-personal-access-token-with-repo-scope>
```

`chmod 600 ~/.netrc`. Bazel reads this automatically.

### CI

For GitHub Actions, set `GITHUB_TOKEN` in the env (the workflow's
built-in `secrets.GITHUB_TOKEN` works if the workflow's repo has
access to the private modules). The credential helper above falls
back to `GITHUB_TOKEN` / `GH_TOKEN` automatically.

## Maintenance

Adding a new version of an existing module, or onboarding a new
module, both go through the [`rels`](https://github.com/fastverk/bazel-registry/tree/main/tools/rels)
CLI shipped from this registry:

```sh
# Cut a new release in the source repo first
cd ~/Workspace/rules_lora
git tag v0.0.2 -m "rules_lora v0.0.2"
git push origin v0.0.2

# Then register it in this registry
cd ~/Workspace/bazel-registry
GH_TOKEN=$(gh auth token) \
  tools/rels/target/release/rels release \
    --repo fastverk/rules_lora \
    --version 0.0.2 \
    --registry-root ~/Workspace/bazel-registry

# Commit + push the registry update
cd ~/Workspace/bazel-registry
git add modules/rules_lora
git commit -m "Register rules_lora v0.0.2"
git push origin main
```

`rels` fetches the GitHub tarball (using `GH_TOKEN` / `GITHUB_TOKEN` /
`gh auth token`), computes the integrity hash, extracts the module's
`MODULE.bazel`, and writes the registry entries. The
[`rels release --help`](https://github.com/fastverk/bazel-registry/tree/main/tools/rels)
docs cover the full flag surface.

## When to use this registry vs citizen-sh

Use this registry for fastverk-owned modules.

For citizen-sh-owned private modules, use
[`citizen-sh/bazel-registry`](https://github.com/citizen-sh/bazel-registry)
first in your registry chain, then this registry, then BCR.

## Module-table maintenance

The module table above is hand-edited until [`botnoc-readme`](https://github.com/fastverk/botnoc)
learns to handle private-tarball registries (it currently only
walks the public registry). Track that work in fastverk/botnoc.

When adding/removing modules manually, keep the table in alphabetical
order and within the `BOTNOC:MODULES_TABLE` markers — the same
splicer convention as the public registry's profile README.
