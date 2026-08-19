"""Resource limits, mirroring rumi core's constants.

These live in their own module rather than in ``_registry`` so the matcher
constructors can enforce them without importing the registry, which imports
them. The limits belong to the types that hold the resource, not to the
loader.

``_registry`` re-exports every name here, so existing imports keep working.
"""

from __future__ import annotations

#: Maximum field matchers in one matcher list.
MAX_FIELD_MATCHERS = 256

# Maximum entries in a single MatcherTree.
#
# Deliberately not MAX_FIELD_MATCHERS, and for a different reason. That limit
# is about evaluation: a list is O(n) per request, so its width is a per-request
# cost. A tree is a keyed lookup, so entry count costs nothing at evaluation
# time — and large routing tables are the entire reason to reach for one. What
# a tree's width costs is memory at config load, which is what this bounds.
MAX_TREE_ENTRIES = 65_536

#: Maximum predicates inside one compound predicate.
MAX_PREDICATES_PER_COMPOUND = 256

#: Maximum length of a literal match pattern (8 KB).
MAX_PATTERN_LENGTH = 8192

#: Maximum length of a regex pattern (4 KB).
#:
#: Length alone does not bound compile cost. ``google-re2`` supplies its own
#: program-size and nested-repetition guards, which is why puma is immune to the
#: compile bomb that bumi needed an explicit budget for (review F-01).
MAX_REGEX_PATTERN_LENGTH = 4096
