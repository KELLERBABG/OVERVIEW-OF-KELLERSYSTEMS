# Contributing to dpsynth

This document provides internal development guidelines for the Differentially Private Synthesizer project.

## Development Setup

```bash
# Install in editable mode with dev dependencies
pip install -e ".[dev]"

# Verify everything works
make test
```

## Project Structure

```
dpsynth/
├── dpsynth/
│   ├── privacy/       # Core DP mechanisms
│   ├── models/        # DP model architectures
│   ├── evaluation/    # Quality & privacy evaluation
│   └── utils/         # Data handling
├── tests/             # Test suite (pytest)
├── docs/              # Documentation
└── examples/          # Runnable example scripts
```

## Workflow

1. Create a branch for the change.
2. **Write tests** for any new functionality (aim for ≥90% coverage on new code).
3. **Run the test suite**:
   ```bash
   python -m pytest tests/ -v --tb=short --cov=dpsynth
   ```
4. **Format code**:
   ```bash
   python -m black dpsynth/ tests/
   ```

## Coding Guidelines

- **Python 3.11+**: Use modern Python features (type hints, dataclasses, pattern matching).
- **Type hints**: All functions should have complete type annotations.
- **Docstrings**: Google-style docstrings for all public APIs.
- **Testing**: Unit tests for individual components, smoke tests for integration paths.
- **DP correctness**: Any new privacy mechanism must include a verification test.

## Adding a New Privacy Mechanism

1. Add the mechanism class to `dpsynth/privacy/`.
2. Export it from `dpsynth/privacy/__init__.py`.
3. Write tests in `tests/test_*.py`.
4. Update the API docs in `docs/api.md`.

## Release Procedure

Releases are fully automated via GitHub Actions and PyPI **trusted
publishing** (OIDC) — no API tokens are stored anywhere in CI.

### Cutting a release

1. Pick the next version following [semver](https://semver.org/).
2. Bump `version` in both `pyproject.toml` and
   `dpsynth/__init__.py::__version__`. They MUST stay in sync — the
   release workflow fails the build if they drift.
3. Add a `## [X.Y.Z] - YYYY-MM-DD` entry to `CHANGELOG.md`. Use
   `<RELEASE_DATE>` in any field the workflow infers from the tag
   only after the tag is pushed.
4. Open a PR; once green, merge to `main`.
5. Tag the merge commit following the regex
   `vX.Y.Z` for stable or `vX.Y.Z-{rc,beta,alpha}.N` for pre-release.
6. Push the tag: `git push origin vX.Y.Z`.

### What happens on push

- `.github/workflows/release.yml` triggers on `vX.*.*` tag pushes.
- `build` job: builds sdist + wheel, runs `twine check`.
- `publish-testpypi`: fires only on `-rc/-beta/-alpha` tags.
- `publish-pypi`: fires only on stable tags.
- `github-release`: always fires, generates notes, attaches artifacts.

### Trust setup (one-time, per-project)

Configure trusted publishing on PyPI / TestPyPI:

- PyPI: Project → Publishing → Add a new pending publisher.
  - Owner: `<your GitHub org or username>`
  - Repository: `<this repo's name>`
  - Workflow filename: `release.yml`
  - Environment name: `pypi`
- TestPyPI: same shape, environment name `testpypi`.

GitHub-side, define matching environments (`pypi`, `testpypi`)
under Settings → Environments. Optional: add a required reviewer on
the `pypi` environment to gate production publishes.

### Rolling back a bad release

- **Yank** the affected version on PyPI (does not delete files).
- For full removal: contact PyPI support (`admin@pypi.org`) — PyPI
  does not allow self-service deletion of releases.
- Force-push a corrected tag only if the release was never
  downloaded by anyone; otherwise yank and cut `vX.Y.Z+1`.
