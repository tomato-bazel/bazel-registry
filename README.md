# fastverk bazel-registry

Bzlmod registry for fastverk Bazel modules.

The registry repo itself is public. It describes where to fetch each
module source tarball. Some module entries point at private GitHub
repos and require auth (covered below).

## Modules

This section is generated from local registry metadata and category
rules in tools/readme/categorization_rules.json.

<!-- BOTNOC:MODULES_TABLE -->
### Core

| Module | Latest | Repository |
|---|---|---|
| [`rules_agentic_ide`](https://github.com/fastverk/rules_agentic_ide) | 0.0.3 | `fastverk/rules_agentic_ide` |
| [`rules_ci_ir`](https://github.com/fastverk/rules_ci_ir) | 0.0.1 | `fastverk/rules_ci_ir` |
| [`rules_lang`](https://github.com/fastverk/rules_lang) | 0.2.0 | `fastverk/rules_lang` |
| [`rules_lean`](https://github.com/fastverk/rules_lean) | 0.3.0-rc1 | `fastverk/rules_lean` |
| [`rules_spec`](https://github.com/fastverk/rules_spec) | 0.5.1 | `fastverk/rules_spec` |

### UI and Rendering

| Module | Latest | Repository |
|---|---|---|
| [`rules_meridian`](https://github.com/mattmarshall/meridian) | 0.2.1 | `mattmarshall/meridian` |
| [`rules_storybook`](https://github.com/fastverk/rules_storybook) | 0.1.0 | `fastverk/rules_storybook` |
| [`rules_vite`](https://github.com/fastverk/rules_vite) | 0.1.0 | `fastverk/rules_vite` |
| [`rules_walkthrough`](https://github.com/fastverk/rules_walkthrough) | 0.1.0 | `fastverk/rules_walkthrough` |
| [`rules_web`](https://github.com/fastverk/rules_web) | 0.0.1 | `fastverk/rules_web` |

### Cloud and Infrastructure

| Module | Latest | Repository |
|---|---|---|
| [`rules_autoconf`](https://github.com/fastverk/rules_autoconf) | 0.1.0 | `fastverk/rules_autoconf` |
| [`rules_cloudformation`](https://github.com/fastverk/rules_cloudformation) | 0.7.0 | `fastverk/rules_cloudformation` |
| [`rules_docker`](https://github.com/fastverk/rules_docker_compose) | 0.2.6 | `fastverk/rules_docker_compose` |
| [`rules_github`](https://github.com/fastverk/rules_github) | 0.1.2 | `fastverk/rules_github` |
| [`rules_gitlab`](https://github.com/fastverk/rules_gitlab) | 0.1.3 | `fastverk/rules_gitlab` |
| [`rules_nextjs`](https://github.com/fastverk/rules_nextjs) | 0.2.0 | `fastverk/rules_nextjs` |
| [`rules_openapi`](https://github.com/fastverk/rules_openapi) | 0.2.1 | `fastverk/rules_openapi` |
| [`rules_postgres`](https://github.com/fastverk/rules_postgres) | 0.4.1 | `fastverk/rules_postgres` |
| [`rules_runpod`](https://github.com/fastverk/rules_runpod) | 0.0.10 | `fastverk/rules_runpod` |
| [`rules_tectonic`](https://github.com/fastverk/rules_tectonic) | 0.2.0 | `fastverk/rules_tectonic` |

### Data and Knowledge

| Module | Latest | Repository |
|---|---|---|
| [`rules_beam`](https://github.com/fastverk/rules_beam) | 0.0.2 | `fastverk/rules_beam` |
| [`rules_huggingface`](https://github.com/fastverk/rules_huggingface) | 0.0.3 | `fastverk/rules_huggingface` |
| [`rules_jena`](https://github.com/fastverk/rules_jena) | 0.3.2 | `fastverk/rules_jena` |
| [`rules_jsonschema`](https://github.com/fastverk/rules_jsonschema) | 0.3.0 | `fastverk/rules_jsonschema` |
| [`rules_puml`](https://github.com/fastverk/rules_puml) | 0.0.2 | `fastverk/rules_puml` |
| [`rules_rdf`](https://github.com/fastverk/rules_rdf) | 0.3.0 | `fastverk/rules_rdf` |
| [`rules_schema_org`](https://github.com/fastverk/rules_schema_org) | 0.0.1 | `fastverk/rules_schema_org` |

### Language and Build Tooling

| Module | Latest | Repository |
|---|---|---|
| [`rules_bibtex`](https://github.com/fastverk/rules_bibtex) | 0.0.6 | `fastverk/rules_bibtex` |
| [`rules_bun`](https://github.com/fastverk/rules_bun) | 0.2.0 | `fastverk/rules_bun` |
| [`rules_cc_cross`](https://github.com/fastverk/rules_cc_cross) | 0.1.0 | `fastverk/rules_cc_cross` |
| [`rules_chrome`](https://github.com/fastverk/rules_chrome) | 0.1.0 | `fastverk/rules_chrome` |
| [`rules_lora`](https://github.com/fastverk/rules_lora) | 0.0.35 | `fastverk/rules_lora` |
| [`rules_mdbook`](https://github.com/fastverk/rules_mdbook) | 0.3.1 | `fastverk/rules_mdbook` |
| [`rules_meson`](https://github.com/fastverk/rules_meson) | 0.0.0 | `fastverk/rules_meson` |
| [`rules_ssh_tui`](https://github.com/fastverk/rules_ssh_tui) | 0.0.5 | `fastverk/rules_ssh_tui` |
| [`rules_uv`](https://github.com/fastverk/rules_uv) | 0.7.4 | `fastverk/rules_uv` |

<!-- /BOTNOC:MODULES_TABLE -->

## Quick start

Add the registry chain to your consumer's .bazelrc:

```
common --registry=https://raw.githubusercontent.com/fastverk/bazel-registry/main/
common --registry=https://bcr.bazel.build/
```

Put this registry before BCR so its entries win for overlapping module
names.

Then declare the dep in MODULE.bazel:

```python
bazel_dep(name = "rules_lora", version = "0.0.1")
```

That is it: no local_path_override needed. The registry resolves the
module's MODULE.bazel + source.json and Bazel fetches the tarball URL.

## Auth

For module entries that point at private GitHub repos, Bazel needs to
forward GitHub credentials when fetching. Two equivalent approaches:

### Option A: Bazel credential helper (recommended)

Project-local and does not pollute global state. Drop a small shell
script into your repo and reference it from .bazelrc. The canonical
implementation lives in
https://github.com/fastverk/rules_postgres/blob/main/tools/credhelper/gh-cred-helper.sh
and forwards gh auth token (or GITHUB_TOKEN / GH_TOKEN) as a bearer
header.

Wire it in .bazelrc:

```
common --credential_helper=*.github.com=%workspace%/tools/credhelper/gh-cred-helper.sh
common --credential_helper=github.com=%workspace%/tools/credhelper/gh-cred-helper.sh
common --credential_helper=raw.githubusercontent.com=%workspace%/tools/credhelper/gh-cred-helper.sh
common --credential_helper=codeload.github.com=%workspace%/tools/credhelper/gh-cred-helper.sh
```

### Option B: ~/.netrc

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

Run chmod 600 ~/.netrc. Bazel reads this automatically.

## Maintenance

Adding a new version or onboarding a new module goes through rels:
https://github.com/fastverk/bazel-registry/tree/main/tools/rels

```sh
cd ~/Workspace/rules_lora
git tag v0.0.2 -m "rules_lora v0.0.2"
git push origin v0.0.2

cd ~/Workspace/bazel-registry
GH_TOKEN=$(gh auth token) \
  tools/rels/target/release/rels release \
    --repo fastverk/rules_lora \
    --version 0.0.2 \
    --registry-root ~/Workspace/bazel-registry

git add modules/rules_lora
git commit -m "Register rules_lora v0.0.2"
git push origin main
```

## When to use this registry

Use this registry for fastverk-owned modules.

## Module-table maintenance

Regenerate README from local metadata with:

```sh
bazel run //:readme.write
```

The companion test target //:readme.write_test detects drift in CI.
