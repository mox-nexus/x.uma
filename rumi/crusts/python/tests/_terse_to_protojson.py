"""Translates the retired terse config dialect into canonical protojson.

Test-only, and deliberately so: the *shipped* schema is protojson, full stop —
`rumi.MatcherConfig`, puma's `xuma._protojson`, bumi's `protojson.ts` know
nothing else. This module exists because these crust tests were written
against the terse dialect's compact literal shape, and rewriting a thousand
lines of nested dict literals by hand is exactly the kind of transcription
work that introduces the bug it's supposed to avoid — the spec/tests
converter had one, caught by a fixture, before it was fixed to raise on a
both-set oneof instead of silently picking one.

So: test bodies keep expressing intent in the terse shape, `_pj()` translates
it once at the boundary, and the actual call into `from_config` is always
real protojson — the crust behavior under test is 100% real; only the
literal syntax written by hand in tests is compact.

Mirrors `spec/tests`'s own migration script line for line. If this drifts from
that script, that is a bug in one of the two, not a feature of either.
"""

from __future__ import annotations

from typing import Any

PREFIX = "type.googleapis.com/"
INPUT_TYPE = PREFIX + "xuma.kv.v1.MapInput"
ACTION_TYPE = PREFIX + "xuma.core.v1.NamedAction"

_VALUE_MATCH = {"Exact": "exact", "Prefix": "prefix", "Suffix": "suffix", "Contains": "contains"}


def _value_match(vm: dict[str, Any]) -> dict[str, Any]:
    (kind, val), = vm.items()
    if kind in _VALUE_MATCH:
        return {_VALUE_MATCH[kind]: val}
    if kind == "Regex":
        return {"safeRegex": {"regex": val}}
    raise ValueError(f"unmapped value_match variant {kind}")


def _typed_config(ref: dict[str, Any], type_url_field: str = "type_url") -> dict[str, Any]:
    url = ref[type_url_field]
    payload = dict(ref.get("config") or {})
    out: dict[str, Any] = {"@type": PREFIX + url}
    out.update(payload)
    return out


def _input_name(type_url: str) -> str:
    return type_url.rsplit(".", 1)[-1]


def _predicate(p: dict[str, Any]) -> dict[str, Any]:
    kind = p["type"]
    if kind == "single":
        sp: dict[str, Any] = {
            "input": {
                "name": _input_name(p["input"]["type_url"]),
                "typedConfig": _typed_config(p["input"]),
            },
        }
        # Emit BOTH when both are set -- they are a oneof, and a passing
        # if/elif here would silently pick one and hide the illegal-config
        # test it is used for.
        if "value_match" in p:
            sp["valueMatch"] = _value_match(p["value_match"])
        if "custom_match" in p:
            sp["customMatch"] = {
                "name": "custom",
                "typedConfig": _typed_config(p["custom_match"]),
            }
        if "valueMatch" not in sp and "customMatch" not in sp:
            raise ValueError("single predicate has neither value_match nor custom_match")
        return {"singlePredicate": sp}
    if kind == "and":
        return {"andMatcher": {"predicate": [_predicate(x) for x in p["predicates"]]}}
    if kind == "or":
        return {"orMatcher": {"predicate": [_predicate(x) for x in p["predicates"]]}}
    if kind == "not":
        return {"notMatcher": _predicate(p["predicate"])}
    raise ValueError(f"unmapped predicate type {kind}")


def _on_match(om: dict[str, Any]) -> dict[str, Any]:
    kind = om["type"]
    if kind == "action":
        name = om["action"]
        return {"action": {"name": name, "typedConfig": {"@type": ACTION_TYPE, "name": name}}}
    if kind == "matcher":
        return {"matcher": _matcher(om["matcher"])}
    raise ValueError(f"unmapped on_match type {kind}")


def _matcher(cfg: dict[str, Any]) -> dict[str, Any]:
    out: dict[str, Any] = {
        "matcherList": {
            "matchers": [
                {"predicate": _predicate(fm["predicate"]), "onMatch": _on_match(fm["on_match"])}
                for fm in cfg["matchers"]
            ]
        }
    }
    if cfg.get("on_no_match") is not None:
        out["onNoMatch"] = _on_match(cfg["on_no_match"])
    return out


def pj(cfg: dict[str, Any], type_url_field: str = "type_url") -> dict[str, Any]:
    """Translate a terse-dialect matcher config dict into canonical protojson."""
    return _matcher(cfg)
