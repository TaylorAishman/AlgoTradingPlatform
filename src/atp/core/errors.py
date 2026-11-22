"""Error types named by the requirements. All derive from `AtpError`."""

from __future__ import annotations


class AtpError(Exception):
    """Base class of every platform error."""


class CalendarError(AtpError):
    """A date is not a stored session (R-012)."""


class ConfigError(AtpError):
    """Invalid or unloadable configuration."""


class DataError(AtpError):
    """Invalid or unavailable data."""


class IngestError(AtpError):
    """An ingest failed after its retries."""


class LookaheadError(AtpError):
    """A computation would use information not available at its as-of time."""


class ModelError(AtpError):
    """A model cannot be estimated from the given inputs."""


class NoForecastError(AtpError):
    """No signal has a usable score."""


class RegistryError(AtpError):
    """Invalid use of the trial registry."""


class SubmissionWindowError(AtpError):
    """An order was submitted outside its allowed submission window."""
