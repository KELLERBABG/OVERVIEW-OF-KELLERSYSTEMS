.PHONY: test install lint clean build clean-docs

# Install the package in editable mode
install:
	pip install -e ".[dev]"

# Run tests with coverage
test:
	python -m pytest tests/ -v --tb=short --cov=dpsynth

# Run tests without coverage (faster)
test-fast:
	python -m pytest tests/ -v --tb=short

# Lint and format check
lint:
	python -m ruff check dpsynth/ tests/
	python -m black --check dpsynth/ tests/ --diff

# Auto-format
format:
	python -m black dpsynth/ tests/

# Build distribution packages
build:
	python -m pip install build
	python -m build

# Clean generated files
clean:
	rm -rf build/ dist/ *.egg-info/ .pytest_cache/ .coverage htmlcov/
	find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true

# Clean documentation build
clean-docs:
	rm -rf docs/_build/ docs/api.md

# Check type hints
typecheck:
	python -m mypy dpsynth/ --ignore-missing-imports || echo "mypy exit: $$?"

# Full CI check
ci: lint test