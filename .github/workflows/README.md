# tomato-bazel automation — the fastverk App control plane

These workflows let the **fastverk GitHub App** (installed on the `tomato-bazel`
org) reproduce, on a schedule, the consistency work that was first done by hand —
via **encapsulated commands + config**, nothing bespoke:

| Concern | Command (encapsulated) | Config (source of truth) | Workflow |
|---|---|---|---|
| Dependency drift → bumps | `rels deps [--write]` (`tools/rels`) | `rules_tomato//bom/versions.json` | `ratchet.yml` |
| Consolidated graph + report (visuals) | `tools/graph/graph.py` | the org's repos | `report.yml` |
| Convention conformance | `rules_jena` SHACL | `rules_tomato//conventions/*.shacl.ttl` | *(planned `conformance.yml`)* |
| Publish a module | `rels release` | `modules/*/source.json` | *(manual / release)* |

## What each does
- **`ratchet.yml`** — weekly (or on demand): audits every org repo's `MODULE.bazel`
  against the BOM and opens a forward-only **bump PR** per behind-drift repo. Merges
  only after each repo's own CI is green. (The same `rels deps` audit can be a
  required PR check so drift can't land.)
- **`report.yml`** — weekly: regenerates the dependency-graph SVG + drift/convention
  report and publishes them to **GitHub Pages** (the health dashboard).

## Installing the fastverk App on tomato-bazel (one-time, owner action)
1. Install the **fastverk** GitHub App on the `tomato-bazel` org, granting it
   repo **contents** + **pull-requests** (and **Pages** for the dashboard).
2. Add two **org secrets**:
   - `FASTVERK_APP_ID` — the App's numeric id.
   - `FASTVERK_APP_PRIVATE_KEY` — the App's PEM private key.
3. Enable **GitHub Pages** (source: GitHub Actions) on this repo for `report.yml`.

The workflows mint a short-lived, org-scoped installation token via
`actions/create-github-app-token` — no long-lived PATs. The App is *auth*; `rels`
+ `graph.py` + the BOM/SHACL are the *logic*. Every step here is runnable locally
the same way (`rels deps --bom …`, `python3 tools/graph/graph.py …`).
