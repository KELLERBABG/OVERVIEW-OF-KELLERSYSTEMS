# ==============================================================================
# Multi-Stage Production Dockerfile for FieldSense Irrigation
# ==============================================================================
FROM python:3.12-slim AS builder

WORKDIR /app

# Install build tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt requirements-dev.txt ./
RUN pip install --no-cache-dir --user -r requirements.txt \
    && pip install --no-cache-dir --user alembic==1.13.2

# Final minimal runtime image
FROM python:3.12-slim AS runner

WORKDIR /app

# Create non-root user
RUN groupadd -g 10001 appuser && \
    useradd -u 10001 -g appuser -s /bin/bash -m appuser

# Copy installed wheels from builder
COPY --from=builder /root/.local /home/appuser/.local
ENV PATH=/home/appuser/.local/bin:$PATH
ENV PYTHONUNBUFFERED=1
ENV PYTHONDONTWRITEBYTECODE=1
# UTF-8 stdio so emoji-bearing advisory logs never raise UnicodeEncodeError
ENV PYTHONUTF8=1
ENV PYTHONIOENCODING=utf-8

# Copy source code and config
COPY --chown=appuser:appuser config/ /app/config/
COPY --chown=appuser:appuser src/ /app/src/
COPY --chown=appuser:appuser migrations/ /app/migrations/
COPY --chown=appuser:appuser alembic.ini /app/alembic.ini

# Set up data directory
RUN mkdir -p /app/data && chown -R appuser:appuser /app/data
ENV DATABASE_URL="sqlite:////app/data/fieldsense.db"

USER appuser

EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/healthz')" || exit 1

# Apply the schema migrations before serving. Alembic is idempotent, so restarting
# the container is safe.
CMD ["sh", "-c", "alembic upgrade head && exec uvicorn src.api.main:app --host 0.0.0.0 --port 8000 --workers 2"]
