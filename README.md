# fastverk premium bazel-registry

Sibling to [`fastverk/bazel-registry`](https://github.com/fastverk/bazel-registry).
Hosts module entries for **private** fastverk Bazel rules: modules
that aren't intended for public consumption but participate in the
same `bazel_dep` resolution graph.

## How to use

Consumers add this registry to their `.bazelrc`:

```
common --registry=https://raw.githubusercontent.com/fastverk/bazel-registry-premium/main/
common --registry=https://raw.githubusercontent.com/fastverk/bazel-registry/main/
common --registry=https://bcr.bazel.build/
```

Then `bazel_dep(name = "rules_lang", ...)` resolves through this
registry's `modules/rules_lang/<version>/` entry instead of needing
a `local_path_override`.

## Auth

The modules' source tarballs are hosted on private GitHub repos.
Bazel needs credentials to fetch them. The simplest setup:

```
# ~/.netrc
machine api.github.com
  login <your-github-username>
  password <gh-token-with-repo-scope>

machine codeload.github.com
  login <your-github-username>
  password <gh-token-with-repo-scope>
```

Or use a Bazel credential helper. See INTERNAL.md.

## Maintenance

Use the `rels` CLI from `fastverk/bazel-registry/tools/rels`:

```sh
rels release --repo fastverk/rules_lang --version 0.0.1 \
  --registry-root /Volumes/Workspace/bazel-registry-premium
```

That writes `modules/rules_lang/0.0.1/{source.json,MODULE.bazel}`
and upserts `modules/rules_lang/metadata.json`.
