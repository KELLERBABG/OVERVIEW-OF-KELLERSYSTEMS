# Release Pipeline — dpsynth

End-to-end release procedure. The pipeline is fully automated once
the trust setup is one-time-configured. No API keys live in CI.

## Quick reference

```
┌─────────────────────────────────────────────────────────────────────┐
│  Tag push ──▶ build ──▶ { publish-testpypi (pre-release tags) │   │
│                          { publish-pypi    (stable tags)     }      │
│                        ──▶ github-release (always)                 │
│                        ──▶ failure-notify (only on red)             │
└─────────────────────────────────────────────────────────────────────┘
```

## Preconditions (one-time, per repo)

### 1. PyPI project: `dpsynth`

Create the project on PyPI (one-time, manual).

### 2. TestPyPI project: `dpsynth`

Create the project on TestPyPI (one-time, manual).

### 3. PyPI trusted publishing

PyPI project → *Publishing* → *Add a new pending publisher*:

| Field            | Value                          |
|------------------|--------------------------------|
| Owner            | `<GitHub org/username>`        |
| Repository name  | `<this repo's name>`           |
| Workflow filename| `release.yml`                  |
| Environment name | `pypi`                         |

### 4. TestPyPI trusted publishing

Same shape, with environment name `testpypi`.

### 5. GitHub environments

`Settings → Environments → New environment`:

- `pypi` — optional required-reviewer to gate production publishes.
- `testpypi` — no reviewers needed.

### 6. (Optional but recommended) Branch protection

`main` branch should require:

- 1+ approving review
- Status check: `CI / test (Python 3.12, ubuntu-latest)`
- Status check: `CI / lint & format (ubuntu-latest)`
- Linear history

The release workflow's `version-match` step inside `build` catches
the `pyproject.toml` ↔ `__init__.py` ↔ git-tag drift before it can
ever ship.

## Cutting a release

```bash
# 1. Pick the next version. Use semver.
NEW="0.3.0"

# 2. Update versions in two files (must stay in sync):
#    pyproject.toml:        version = "$NEW"
#    dpsynth/__init__.py:   __version__ = "$NEW"

# 3. Add a CHANGELOG.md entry:
cat >> CHANGELOG.md <<EOF

## [$NEW] - $(date -u +%Y-%m-%d)

### Changed
- ...

### Added
- ...
EOF

# 4. Commit + push + PR:
git checkout -b "release/$NEW"
git add pyproject.toml dpsynth/__init__.py CHANGELOG.md
git commit -m "chore: bump to $NEW"
git push -u origin "release/$NEW"
# open & merge the PR via your normal review flow

# 5. Tag and push:
git checkout main
git pull
git tag -a "v$NEW" -m "dpsynth v$NEW — <short summary>"
git push origin "v$NEW"
```

## What the pipeline does

On `vX.Y.Z` tag push, `.github/workflows/release.yml` runs:

1. **`build`** (always)
   - Checks out the repo at the tag commit.
   - Verifies `pyproject.toml` `version` equals the tag (`v$NEW` ⇒ `$NEW`).
   - Installs `build`, runs `python -m build` → produces `dist/*.tar.gz`, `dist/*.whl`.
   - Runs `twine check dist/*` to validate against PyPI rules.
   - Uploads `dist/` as a workflow artifact.

2. **`publish-testpypi`** (only when the tag contains `-`)
   - Downloads the `dist/` artifact.
   - Publishes to TestPyPI via trusted publishing (OIDC).
   - Requires GitHub environment `testpypi`.
   - URL: <https://test.pypi.org/project/dpsynth/>.

3. **`publish-pypi`** (only when the tag has no `-`)
   - Downloads the `dist/` artifact.
   - Publishes to PyPI via trusted publishing (OIDC).
   - Requires GitHub environment `pypi`.
   - URL: <https://pypi.org/project/dpsynth/>.

4. **`github-release`** (always)
   - Creates a GitHub Release at tag `vX.Y.Z`.
   - Auto-generates notes from merged PRs.
   - Attaches `dist/*.tar.gz` and `dist/*.whl`.
   - Marks pre-release flags when the tag ships with `-rc/-beta/-alpha`.

5. **`failure-notify`** (only when something broke)
   - Currently a log-only stub; extend with a Slack/email ping
     if/when on-call coverage expands.

## Rolling back

- **Yank** the affected version on PyPI: it remains installable for
  those who already downloaded it but disappears from `pip install`
  for everyone else. Use this for everything short of a security
  regression.
- **Force-push a corrected tag** only if the release was never
  downloaded by anyone. This physically rewrites the tag pointer.
- **Yank + cut `vX.Y.Z+1`** is the safe path: yank the bad version
  and ship a fix-forward release.

## Local dry-run (before tagging)

```bash
# Build what would ship:
python -m pip install build
python -m build

# Validate against PyPI's rules without publishing:
python -m pip install twine
twine check dist/*

# Inspect what would be packaged:
tar -tzf dist/dpsynth-0.2.0.tar.gz | head
```

If `twine check` reports warnings (notably about the `Proprietary`
classifier's metadata), it's a confirmation that the package would
ship — not a blocker — but warnings that mention missing files or
mismatched metadata ARE blockers.
