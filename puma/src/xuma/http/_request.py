"""HttpRequest — Simple HTTP request context for matching.

Holds method, path (without query string), headers (case-insensitive),
and query parameters (parsed from the raw path).
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True, slots=True)
class HttpRequest:
    """HTTP request context for matching.

    The path should be provided as-is from the wire (may include query string).
    Query parameters are automatically parsed and the path is cleaned.

    Headers are stored with lowercased keys for case-insensitive lookup.
    """

    method: str = "GET"
    raw_path: str = "/"
    headers: dict[str, str] = field(default_factory=dict)

    # Computed fields — parsed from raw_path
    _clean_path: str = field(init=False, repr=False)
    _query_params: dict[str, str] = field(init=False, repr=False)
    _lower_headers: dict[str, str] = field(init=False, repr=False)

    def __post_init__(self) -> None:
        # Parse query string from path
        if "?" in self.raw_path:
            path, query_string = self.raw_path.split("?", 1)
            params: dict[str, str] = {}
            for part in query_string.split("&"):
                if "=" in part:
                    k, v = part.split("=", 1)
                    params[k] = v
                elif part:
                    params[part] = ""
            object.__setattr__(self, "_clean_path", path)
            object.__setattr__(self, "_query_params", params)
        else:
            object.__setattr__(self, "_clean_path", self.raw_path)
            object.__setattr__(self, "_query_params", {})

        # Lowercase header keys for case-insensitive lookup
        object.__setattr__(
            self,
            "_lower_headers",
            {k.lower(): v for k, v in self.headers.items()},
        )

    @property
    def path(self) -> str:
        """Path without query string."""
        return self._clean_path

    @property
    def query_params(self) -> dict[str, str]:
        """Parsed query parameters."""
        return self._query_params

    def header(self, name: str) -> str | None:
        """Get a header value by name (case-insensitive)."""
        return self._lower_headers.get(name.lower())

    def query_param(self, name: str) -> str | None:
        """Get a query parameter by name."""
        return self._query_params.get(name)
