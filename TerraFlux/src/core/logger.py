"""Structured logging configuration for FieldSense Irrigation."""
import logging
import sys
from .config import settings

def _force_utf8_stdio() -> None:
    """Guarantee UTF-8 capable stdout/stderr.

    Advisory messages contain emoji (✅/💧/🚨). On a Windows console using the
    default cp1252 codec the StreamHandler raises UnicodeEncodeError on every
    dispatch, flooding stderr with logging errors. Reconfigure defensively so
    the platform default never breaks logging.
    """
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, 'reconfigure', None)
        if reconfigure is None:
            continue
        try:
            reconfigure(encoding='utf-8', errors='replace')
        except Exception:
            # Stream may be a non-reconfigurable wrapper (e.g. under pytest capture)
            pass

def setup_logger(name: str = 'fieldsense') -> logging.Logger:
    logger = logging.getLogger(name)
    if not logger.handlers:
        _force_utf8_stdio()
        logger.setLevel(getattr(logging, settings.LOG_LEVEL.upper(), logging.INFO))
        handler = logging.StreamHandler(sys.stdout)
        handler.setLevel(getattr(logging, settings.LOG_LEVEL.upper(), logging.INFO))
        formatter = logging.Formatter(
            fmt='%(asctime)s [%(levelname)s] [%(name)s] %(message)s',
            datefmt='%Y-%m-%d %H:%M:%S'
        )
        handler.setFormatter(formatter)
        logger.addHandler(handler)
    return logger

logger = setup_logger()
