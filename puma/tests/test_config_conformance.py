"""Config conformance tests for puma.

Loads YAML fixtures from spec/tests/06_config/ and runs them through
puma's registry config loading path — the same fixtures that rumi,
bumi, and both crusty bindings must also pass.

Run with: cd puma && uv run pytest tests/test_config_conformance.py -v
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest
import yaml

from puma import (
    ConfigParseError,
    MatcherError,
    RegistryBuilder,
    parse_matcher_config,
)
from puma.testing import register

SPEC_DIR = Path(__file__).resolve().parent.parent.parent / "spec" / "tests"
CONFIG_DIR = SPEC_DIR / "06_config"


def _make_registry():  # noqa: ANN202
    """Build a registry with the test domain."""
    builder = RegistryBuilder()
    builder = register(builder)
    return builder.build()


def _load_config_fixtures() -> list[dict[str, Any]]:
    """Load all config fixture YAML files."""
    fixtures: list[dict[str, Any]] = []
    if not CONFIG_DIR.exists():
        return fixtures

    for yaml_file in sorted(CONFIG_DIR.glob("*.yaml")):
        with yaml_file.open() as f:
            for doc in yaml.safe_load_all(f):
                if doc is None:
                    continue
                doc["_source"] = yaml_file.name
                fixtures.append(doc)

    return fixtures


def _fixture_id(fixture: dict[str, Any]) -> str:
    """Generate a readable test ID from a fixture."""
    source = fixture.get("_source", "unknown")
    name = fixture.get("name", "unnamed")
    return f"{source}::{name}"


# Separate positive and error fixtures
_all_fixtures = _load_config_fixtures()
_positive_fixtures = [f for f in _all_fixtures if not f.get("expect_error", False)]
_error_fixtures = [f for f in _all_fixtures if f.get("expect_error", False)]


_positive_ids = [_fixture_id(f) for f in _positive_fixtures]


@pytest.mark.parametrize("fixture", _positive_fixtures, ids=_positive_ids)
def test_config_positive(fixture: dict[str, Any]) -> None:
    """Positive config fixture: parse, load, and evaluate must succeed."""
    registry = _make_registry()

    config = parse_matcher_config(fixture["config"])
    matcher = registry.load_matcher(config)

    for case in fixture.get("cases", []):
        ctx = {str(k): str(v) for k, v in case["context"].items()}
        actual = matcher.evaluate(ctx)
        expected = case["expect"]
        assert actual == expected, (
            f"Fixture '{fixture['name']}' case '{case['name']}': "
            f"expected {expected!r}, got {actual!r}"
        )


@pytest.mark.parametrize("fixture", _error_fixtures, ids=[_fixture_id(f) for f in _error_fixtures])
def test_config_error(fixture: dict[str, Any]) -> None:
    """Error config fixture: either parse or load must fail."""
    registry = _make_registry()

    try:
        config = parse_matcher_config(fixture["config"])
    except (ConfigParseError, KeyError, TypeError, ValueError):
        # Parse error — expected
        return

    # Parse succeeded, loading must fail
    with pytest.raises(MatcherError):
        registry.load_matcher(config)
