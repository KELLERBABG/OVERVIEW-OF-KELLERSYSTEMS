"""Timezone helpers.

SQLite has no native timezone support: SQLAlchemy stores naive UTC strings and the
``tzinfo`` is dropped when a row is read back. Comparing such a value with an
aware ``datetime.now(timezone.utc)`` raises a TypeError, so every comparison between a
persisted timestamp and "now" has to go through :func:`as_utc`.
"""
from datetime import datetime, timezone
from typing import Optional


def as_utc(value: Optional[datetime]) -> Optional[datetime]:
    """Return a timezone-aware UTC datetime.

    Naive values are assumed to be UTC (that is how they were written).
    """
    if value is None:
        return None
    if value.tzinfo is None:
        return value.replace(tzinfo=timezone.utc)
    return value.astimezone(timezone.utc)


def hours_between(later: Optional[datetime], earlier: Optional[datetime]) -> float:
    """Hours between two timestamps, safe for naive/aware mixes."""
    a = as_utc(later)
    b = as_utc(earlier)
    if a is None or b is None:
        return 0.0
    return (a - b).total_seconds() / 3600.0
