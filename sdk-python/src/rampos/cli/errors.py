"""Stable CLI error types and exit codes."""

from __future__ import annotations


class CliError(Exception):
    exit_code = 1


class CliUsageError(CliError):
    exit_code = 2


class CliAuthError(CliError):
    exit_code = 3


class CliTransportError(CliError):
    exit_code = 4


class CliHttpError(CliError):
    exit_code = 5

    def __init__(self, message: str, status_code: int, body: str = "") -> None:
        super().__init__(message)
        self.status_code = status_code
        self.body = body


def exit_code_for_error(error: BaseException) -> int:
    if isinstance(error, CliError):
        return error.exit_code
    return 1
