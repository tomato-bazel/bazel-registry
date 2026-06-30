# constellation automation — the fastverk App control plane

These workflows let the **fastverk GitHub App** reproduce, on a schedule, the
consistency work that was first done by hand — via **encapsulated commands +
config**, nothing bespoke. They span **both orgs** — `tomato-bazel` (the Bazel
distro) and `fastverk` (products + services, incl. `fastverk/agent`) — minting one
installation token per org, so the graph, the ratchet, and the conformance gate
are all cross-org.

| Concern | Command (encapsulated) | Config (source of truth) | Workflow |
|---|---|---|---|
| Dependency drift → bumps | `rels deps [--write]` (`tools/rels`) | `rules_tomato//bom/versions.json` | `ratchet.yml` |
| Consolidated graph + report (visuals) | `tools/graph/graph.py` | both orgs' repos | `report.yml` |
| Convention conformance | SHACL (`pyshacl`; rules_jena in-Bazel) | `rules_tomato//conventions/*.shacl.ttl` | `conformance.yml` |
| Publish a module | `rels release` | `modules/*/source.json` | *(manual / release)* |

## What each does
- **`ratchet.yml`** — weekly (or on demand): audits every repo's `MODULE.bazel`
  across both orgs against the BOM and opens a forward-only **bump PR** per
  behind-drift repo, in that repo's own org. Merges only after each repo's CI is
  green. (The same `rels deps` audit can be a required PR check so drift can't land.)
- **`report.yml`** — weekly: regenerates the dependency-graph SVG + drift/convention
  report for the whole constellation and publishes it to **GitHub Pages** (the
  health dashboard).
- **`conformance.yml`** — weekly: projects the constellation to RDF and validates it
  against the SHACL convention contract. **Advisory** today (reports to the job
  summary); flip `continue-on-error` off to make it a hard gate once conventions are
  backfilled.

## Installing the fastverk App (one-time, owner action) — on BOTH orgs
1. Install the **fastverk** GitHub App on `tomato-bazel` **and** `fastverk`, granting
   repo **contents** + **pull-requests** (and **Pages** on this repo for the dashboard).
2. Add two **org secrets** (on the org that runs these workflows):
   - `FASTVERK_APP_ID` — the App's numeric id.
   - `FASTVERK_APP_PRIVATE_KEY` — the App's PEM private key.
3. Enable **GitHub Pages** (source: GitHub Actions) on this repo for `report.yml`.

The workflows mint a short-lived, org-scoped installation token via
`actions/create-github-app-token` — no long-lived PATs. The App is *auth*; `rels`
+ `graph.py` + the BOM/SHACL are the *logic*. Every step here is runnable locally
the same way (`rels deps --bom …`, `python3 tools/graph/graph.py …`).
