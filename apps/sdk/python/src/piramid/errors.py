"""The error the client raises when the server refuses a request."""

from __future__ import annotations

import httpx


class PiramidError(Exception):
    """A request the server answered with an error, carrying its message and HTTP status."""

    def __init__(self, message: str, status: int) -> None:
        super().__init__(f"{status}: {message}")
        self.message = message
        self.status = status

    @classmethod
    def from_response(cls, response: httpx.Response) -> "PiramidError":
        """The error a failed response describes."""
        try:
            message = response.json().get("error", response.text)
        except ValueError:
            message = response.text or response.reason_phrase
        return cls(message, response.status_code)
