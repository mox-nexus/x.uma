"""Tests for xuma-crust HookMatcher.

Covers:
- Conformance: same input → same result as Rust rumi::claude
- Security: fail-closed, input limits, invalid regex
- Trace: debugging visibility
"""

import pytest

from xuma_crust import HookMatcher, HookMatch, StringMatch


# ═══════════════════════════════════════════════════════════════════════════════
# Conformance: must produce identical results to rumi::claude Rust tests
# ═══════════════════════════════════════════════════════════════════════════════


class TestConformance:
    """Mirror rumi::claude compiler::tests scenarios through FFI."""

    def test_event_match(self):
        matcher = HookMatcher.compile(
            [HookMatch(event="PreToolUse")],
            action="matched",
        )
        assert matcher.evaluate(event="PreToolUse") == "matched"
        assert matcher.evaluate(event="PostToolUse") is None

    def test_tool_exact_match(self):
        matcher = HookMatcher.compile(
            [HookMatch(tool_name="Bash")],
            action="matched",
        )
        assert matcher.evaluate(event="PreToolUse", tool_name="Bash") == "matched"
        assert matcher.evaluate(event="PreToolUse", tool_name="Write") is None

    def test_tool_prefix_match(self):
        matcher = HookMatcher.compile(
            [HookMatch(tool_name=StringMatch.prefix("mcp__"))],
            action="matched",
        )
        assert matcher.evaluate(event="PreToolUse", tool_name="mcp__db__delete") == "matched"
        assert matcher.evaluate(event="PreToolUse", tool_name="Write") is None

    def test_tool_regex_match(self):
        matcher = HookMatcher.compile(
            [HookMatch(tool_name=StringMatch.regex(r"^(Write|Edit)$"))],
            action="matched",
        )
        assert matcher.evaluate(event="PreToolUse", tool_name="Write") == "matched"
        assert matcher.evaluate(event="PreToolUse", tool_name="Edit") == "matched"
        assert matcher.evaluate(event="PreToolUse", tool_name="Bash") is None

    def test_argument_contains_match(self):
        matcher = HookMatcher.compile(
            [HookMatch(arguments=[("command", StringMatch.contains("rm -rf"))])],
            action="blocked",
        )
        result = matcher.evaluate(
            event="PreToolUse",
            tool_name="Bash",
            arguments={"command": "sudo rm -rf /"},
        )
        assert result == "blocked"

    def test_argument_missing_returns_no_match(self):
        matcher = HookMatcher.compile(
            [HookMatch(arguments=[("command", StringMatch.contains("rm"))])],
            action="blocked",
        )
        # No arguments provided
        assert matcher.evaluate(event="PreToolUse", tool_name="Bash") is None

    def test_combined_event_and_tool(self):
        matcher = HookMatcher.compile(
            [HookMatch(event="PreToolUse", tool_name="Bash")],
            action="matched",
        )
        assert matcher.evaluate(event="PreToolUse", tool_name="Bash") == "matched"
        # Wrong event
        assert matcher.evaluate(event="PostToolUse", tool_name="Bash") is None
        # Wrong tool
        assert matcher.evaluate(event="PreToolUse", tool_name="Write") is None

    def test_cwd_prefix_match(self):
        matcher = HookMatcher.compile(
            [HookMatch(cwd=StringMatch.prefix("/home/user"))],
            action="matched",
        )
        assert matcher.evaluate(event="PreToolUse", cwd="/home/user/project") == "matched"
        assert matcher.evaluate(event="PreToolUse", cwd="/tmp") is None

    def test_git_branch_match(self):
        matcher = HookMatcher.compile(
            [HookMatch(git_branch="main")],
            action="matched",
        )
        assert matcher.evaluate(event="PreToolUse", git_branch="main") == "matched"
        assert matcher.evaluate(event="PreToolUse", git_branch="dev") is None
        # No branch
        assert matcher.evaluate(event="PreToolUse") is None

    def test_combined_all_fields(self):
        matcher = HookMatcher.compile(
            [HookMatch(
                event="PreToolUse",
                tool_name="Bash",
                arguments=[("command", StringMatch.contains("rm"))],
                cwd=StringMatch.prefix("/home"),
                git_branch="main",
            )],
            action="blocked",
        )
        result = matcher.evaluate(
            event="PreToolUse",
            tool_name="Bash",
            arguments={"command": "rm -rf /"},
            cwd="/home/user",
            git_branch="main",
        )
        assert result == "blocked"

    def test_multiple_rules_or(self):
        """Multiple rules are ORed: first match wins."""
        matcher = HookMatcher.compile(
            [
                HookMatch(tool_name="Bash"),
                HookMatch(tool_name="Write"),
            ],
            action="matched",
        )
        assert matcher.evaluate(event="PreToolUse", tool_name="Bash") == "matched"
        assert matcher.evaluate(event="PreToolUse", tool_name="Write") == "matched"
        assert matcher.evaluate(event="PreToolUse", tool_name="Read") is None

    def test_fallback_action(self):
        matcher = HookMatcher.compile(
            [HookMatch(tool_name="Bash")],
            action="deny",
            fallback="allow",
        )
        assert matcher.evaluate(event="PreToolUse", tool_name="Bash") == "deny"
        assert matcher.evaluate(event="PreToolUse", tool_name="Write") == "allow"

    def test_match_all_explicit(self):
        """match_all=True catches everything."""
        matcher = HookMatcher.compile(
            [HookMatch(match_all=True)],
            action="catch_all",
        )
        assert matcher.evaluate(event="PreToolUse", tool_name="anything") == "catch_all"
        assert matcher.evaluate(event="Stop") == "catch_all"


# ═══════════════════════════════════════════════════════════════════════════════
# Scenarios: real-world Claude Code hook patterns
# ═══════════════════════════════════════════════════════════════════════════════


class TestScenarios:
    """Real-world usage patterns from Claude Code hooks."""

    def test_block_dangerous_bash(self):
        """Block rm -rf commands."""
        matcher = HookMatcher.compile(
            [HookMatch(
                event="PreToolUse",
                tool_name="Bash",
                arguments=[("command", StringMatch.contains("rm -rf"))],
            )],
            action="deny",
            fallback="allow",
        )
        # Dangerous
        assert matcher.evaluate(
            event="PreToolUse",
            tool_name="Bash",
            arguments={"command": "rm -rf /important"},
        ) == "deny"
        # Safe
        assert matcher.evaluate(
            event="PreToolUse",
            tool_name="Bash",
            arguments={"command": "ls -la"},
        ) == "allow"

    def test_block_mcp_deletes(self):
        """Block all MCP delete operations."""
        matcher = HookMatcher.compile(
            [HookMatch(tool_name=StringMatch.regex(r"^mcp__.*__delete"))],
            action="deny",
            fallback="allow",
        )
        assert matcher.evaluate(
            event="PreToolUse",
            tool_name="mcp__db__delete_row",
        ) == "deny"
        assert matcher.evaluate(
            event="PreToolUse",
            tool_name="mcp__db__read_row",
        ) == "allow"

    def test_branch_protection(self):
        """Block writes on main branch."""
        matcher = HookMatcher.compile(
            [HookMatch(
                event="PreToolUse",
                tool_name=StringMatch.regex(r"^(Write|Edit)$"),
                git_branch="main",
            )],
            action="deny",
            fallback="allow",
        )
        # On main: blocked
        assert matcher.evaluate(
            event="PreToolUse",
            tool_name="Write",
            git_branch="main",
        ) == "deny"
        # On feature branch: allowed
        assert matcher.evaluate(
            event="PreToolUse",
            tool_name="Write",
            git_branch="feat/test",
        ) == "allow"

    def test_all_hook_events(self):
        """All 9 hook event types are recognized."""
        events = [
            "PreToolUse", "PostToolUse", "Stop", "SubagentStop",
            "UserPromptSubmit", "SessionStart", "SessionEnd",
            "PreCompact", "Notification",
        ]
        for event in events:
            matcher = HookMatcher.compile(
                [HookMatch(event=event)],
                action="matched",
            )
            assert matcher.evaluate(event=event) == "matched", f"Failed for {event}"


# ═══════════════════════════════════════════════════════════════════════════════
# Security: fail-closed, input limits, error handling (Vector requirements)
# ═══════════════════════════════════════════════════════════════════════════════


class TestSecurity:
    """Vector arch-guild security requirements."""

    def test_empty_match_rejected(self):
        """V-BYPASS-1: Empty HookMatch without match_all → error."""
        with pytest.raises(ValueError, match="empty HookMatch"):
            HookMatcher.compile([HookMatch()], action="blocked")

    def test_empty_match_allowed_with_flag(self):
        """match_all=True explicitly allows catch-all."""
        matcher = HookMatcher.compile(
            [HookMatch(match_all=True)],
            action="allow",
        )
        assert matcher.evaluate(event="PreToolUse") == "allow"

    def test_invalid_regex_raises(self):
        """Invalid regex → clear ValueError, not panic."""
        with pytest.raises(ValueError, match="regex"):
            HookMatcher.compile(
                [HookMatch(tool_name=StringMatch.regex("[invalid"))],
                action="blocked",
            )

    def test_unknown_event_raises(self):
        """Unknown event string → ValueError."""
        with pytest.raises(ValueError, match="unknown hook event"):
            matcher = HookMatcher.compile(
                [HookMatch(event="PreToolUse")],
                action="test",
            )
            matcher.evaluate(event="NotAnEvent")

    def test_unknown_event_in_config_raises(self):
        """Unknown event in config → ValueError."""
        with pytest.raises(ValueError, match="unknown hook event"):
            HookMatcher.compile(
                [HookMatch(event="FakeEvent")],
                action="test",
            )

    def test_too_many_rules_raises(self):
        """MAX_RULES exceeded → ValueError."""
        rules = [HookMatch(event="PreToolUse") for _ in range(257)]
        with pytest.raises(ValueError, match="too many rules"):
            HookMatcher.compile(rules, action="blocked")

    def test_too_many_arguments_raises(self):
        """MAX_ARGUMENTS exceeded → ValueError."""
        args = [(f"arg{i}", StringMatch.exact("v")) for i in range(65)]
        with pytest.raises(ValueError, match="too many arguments"):
            HookMatcher.compile(
                [HookMatch(event="PreToolUse", arguments=args)],
                action="blocked",
            )

    def test_oversized_pattern_raises(self):
        """MAX_PATTERN_LENGTH exceeded → ValueError."""
        big = "a" * 8193
        with pytest.raises(ValueError, match="exceeds limit"):
            HookMatcher.compile(
                [HookMatch(tool_name=StringMatch.exact(big))],
                action="blocked",
            )

    def test_oversized_regex_raises(self):
        """MAX_REGEX_PATTERN_LENGTH exceeded → ValueError."""
        big = "a" * 4097
        with pytest.raises(ValueError, match="exceeds limit"):
            HookMatcher.compile(
                [HookMatch(tool_name=StringMatch.regex(big))],
                action="blocked",
            )


# ═══════════════════════════════════════════════════════════════════════════════
# Trace: debugging visibility (opt-in per Vector)
# ═══════════════════════════════════════════════════════════════════════════════


class TestTrace:
    """Trace evaluation for debugging."""

    def test_trace_result_matches_evaluate(self):
        """INV: trace.result == evaluate() for same input."""
        matcher = HookMatcher.compile(
            [HookMatch(event="PreToolUse", tool_name="Bash")],
            action="block",
            fallback="allow",
        )
        eval_result = matcher.evaluate(event="PreToolUse", tool_name="Bash")
        trace = matcher.trace(event="PreToolUse", tool_name="Bash")
        assert trace.result == eval_result

    def test_trace_has_steps(self):
        """Trace shows evaluation steps."""
        matcher = HookMatcher.compile(
            [HookMatch(event="PreToolUse", tool_name="Bash")],
            action="block",
        )
        trace = matcher.trace(event="PreToolUse", tool_name="Bash")
        assert len(trace.steps) > 0
        assert trace.steps[0].matched is True

    def test_trace_no_match_uses_fallback(self):
        """Trace correctly indicates fallback usage."""
        matcher = HookMatcher.compile(
            [HookMatch(tool_name="Bash")],
            action="block",
            fallback="allow",
        )
        trace = matcher.trace(event="PreToolUse", tool_name="Write")
        assert trace.result == "allow"
        assert trace.used_fallback is True

    def test_trace_step_repr(self):
        """Trace steps have readable repr."""
        matcher = HookMatcher.compile(
            [HookMatch(event="PreToolUse")],
            action="matched",
        )
        trace = matcher.trace(event="PreToolUse")
        step = trace.steps[0]
        assert "TraceStep" in repr(step)
        assert str(step.index) in repr(step)


# ═══════════════════════════════════════════════════════════════════════════════
# DX: convenience features (Ace recommendations)
# ═══════════════════════════════════════════════════════════════════════════════


class TestDeveloperExperience:
    """DX features from Ace arch-guild review."""

    def test_bare_string_is_exact_match(self):
        """Bare string → exact match (Ace convenience)."""
        matcher = HookMatcher.compile(
            [HookMatch(tool_name="Bash")],
            action="matched",
        )
        assert matcher.evaluate(event="PreToolUse", tool_name="Bash") == "matched"
        assert matcher.evaluate(event="PreToolUse", tool_name="BashScript") is None

    def test_string_match_constructors(self):
        """All StringMatch factory methods work."""
        for factory, arg in [
            (StringMatch.exact, "Bash"),
            (StringMatch.prefix, "mcp__"),
            (StringMatch.suffix, ".rs"),
            (StringMatch.contains, "rm"),
            (StringMatch.regex, r"^Bash$"),
        ]:
            sm = factory(arg)
            # Just verify it's usable in HookMatch
            HookMatcher.compile(
                [HookMatch(tool_name=sm)],
                action="test",
            )

    def test_matcher_repr(self):
        """Matcher has readable repr."""
        matcher = HookMatcher.compile(
            [HookMatch(event="PreToolUse")],
            action="test",
        )
        assert "HookMatcher" in repr(matcher)

    def test_hook_match_repr(self):
        """HookMatch has readable repr."""
        hm = HookMatch(event="PreToolUse", tool_name="Bash")
        assert "HookMatch" in repr(hm)

    def test_string_match_repr(self):
        """StringMatch has readable repr."""
        assert "exact" in repr(StringMatch.exact("Bash"))
        assert "prefix" in repr(StringMatch.prefix("mcp__"))
        assert "regex" in repr(StringMatch.regex(r"^Bash$"))


# ═══════════════════════════════════════════════════════════════════════════════
# Session semantics (Dijkstra: preserve session_id="" vs git_branch=None)
# ═══════════════════════════════════════════════════════════════════════════════


class TestSessionSemantics:
    """Dijkstra requirement: preserve semantic distinctions."""

    def test_session_id_empty_string_matches(self):
        """session_id="" is a valid value (anonymous), not absent."""
        matcher = HookMatcher.compile(
            [HookMatch(session_id=StringMatch.exact(""))],
            action="matched",
        )
        # Empty string should match
        assert matcher.evaluate(event="PreToolUse", session_id="") == "matched"

    def test_git_branch_none_vs_present(self):
        """git_branch=None (not in repo) vs git_branch="main" (in repo)."""
        matcher = HookMatcher.compile(
            [HookMatch(git_branch="main")],
            action="matched",
        )
        # Present and matching
        assert matcher.evaluate(event="PreToolUse", git_branch="main") == "matched"
        # Absent (not in a git repo)
        assert matcher.evaluate(event="PreToolUse") is None
