# fastverk premium bazel-registry

Bzlmod registry for the **private** fastverk Bazel rules. Sibling
to [`fastverk/bazel-registry`](https://github.com/fastverk/bazel-registry)
(which hosts the public modules).

The registry repo itself is public — it just describes WHERE to fetch
each module. The source tarballs the entries point at live in private
GitHub repos and require auth (covered below).

## Modules

<!-- BOTNOC:MODULES_TABLE -->
| Module | Latest | Description |
|---|---|---|
| [`rules_lang`](https://github.com/fastverk/rules_lang) | 0.0.0 | Polyglot AST round-trip + translator engine (token-tree substitution → AST diff). Provides `Polyglot.Sql.Ast` (the generic 32-ctor SQL AST that `Pg.Ast` extends via the Pattern-A `Ext` slot). |
| [`rules_lora`](https://github.com/fastverk/rules_lora) | 0.0.1 | Bazel-native LoRA fine-tuning. Four declarative macros: `lora_dataset`, `lora_recipe`, `lora_train`, `expert_manifest`. RunPod / local-CPU backends; torchtune-rendered recipes. |
| [`rules_meson`](https://github.com/fastverk/rules_meson) | 0.0.0 | Hermetic meson + ninja toolchain for Bazel. `meson_configure` runs `meson setup` as a build action; consumers get a deterministic `compile_commands.json`. |
<!-- /BOTNOC:MODULES_TABLE -->

(More modules from the embedded-systems family — rules_sel4, rules_chisel,
rules_microkit, etc. — are pending a follow-up sweep into this registry.)

## Quick start

Add the registry chain to your consumer's `.bazelrc`:

```
common --registry=https://raw.githubusercontent.com/fastverk/bazel-registry-premium/main/
common --registry=https://raw.githubusercontent.com/fastverk/bazel-registry/main/
common --registry=https://bcr.bazel.build/
```

Premium first so its entries win over BCR for the same module name.

Then declare the dep in `MODULE.bazel`:

```python
bazel_dep(name = "rules_lora", version = "0.0.1")
```

That's it — no `local_path_override` needed. The registry resolves
the module's `MODULE.bazel` + `source.json`; Bazel fetches the tarball
via the source.json URL (which requires GitHub auth — see below).

## Auth

Source tarballs live in private GitHub repos under `fastverk/`. Bazel
needs to forward GitHub credentials when fetching. Two equivalent
approaches:

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
CLI shipped from the public registry. Same tool, different
`--registry-root`:

```sh
# Cut a new release in the source repo first
cd ~/Workspace/rules_lora
git tag v0.0.2 -m "rules_lora v0.0.2"
git push origin v0.0.2

# Then register it in the premium registry
cd ~/Workspace/bazel-registry
GH_TOKEN=$(gh auth token) \
  tools/rels/target/release/rels release \
    --repo fastverk/rules_lora \
    --version 0.0.2 \
    --registry-root ~/Workspace/bazel-registry-premium

# Commit + push the registry update
cd ~/Workspace/bazel-registry-premium
git add modules/rules_lora
git commit -m "Register rules_lora v0.0.2"
git push origin main
```

`rels` fetches the GitHub tarball (using `GH_TOKEN` / `GITHUB_TOKEN` /
`gh auth token`), computes the integrity hash, extracts the module's
`MODULE.bazel`, and writes the registry entries. The
[`rels release --help`](https://github.com/fastverk/bazel-registry/tree/main/tools/rels)
docs cover the full flag surface.

## When to use this registry vs the public one

Add a module here if it's:
- Sourced from a private fastverk repo (the tarball URL needs auth)
- Premium-tier (paid consumer access only)
- Not yet ready for public consumption (early-stage work; private
  iteration before promoting to the public registry)

Otherwise, use [`fastverk/bazel-registry`](https://github.com/fastverk/bazel-registry).

A module can move from premium to public later — just register the
same version in the public registry, set the source.json URL to the
public-repo tarball, and update consumer pins.

## Module-table maintenance

The module table above is hand-edited until [`botnoc-readme`](https://github.com/fastverk/botnoc)
learns to handle private-tarball registries (it currently only
walks the public registry). Track that work in fastverk/botnoc.

When adding/removing modules manually, keep the table in alphabetical
order and within the `BOTNOC:MODULES_TABLE` markers — the same
splicer convention as the public registry's profile README.
