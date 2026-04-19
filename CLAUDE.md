# Development Rules - NON-NEGOTIABLE

**Last Updated:** 2026-04-18
**Status**: MANDATORY for ALL code changes

---

## Rule #1: NEVER GUESS - ALWAYS VERIFY

Before ANY code change:

```bash
# 1. READ the source code first
read_file /path/to/file.rs

# 2. VERIFY with compiler/tests
cargo test
cargo check
```

**FORBIDDEN**:
- ❌ Assuming what a function does without reading it
- ❌ Writing code based on "how it should work"
- ❌ Using grep to "find" symbols (use specific search tools instead)

---

## Rule #2: TDD - PROVE IT FIRST

1. Write a test that FAILS for the expected reason
2. Show the failure output
3. Fix the code
4. Show the test now passes

```bash
# Step 1: Run test - should FAIL with expected error
cargo test test_name

# Step 2: Fix code

# Step 3: Run test again - should PASS
cargo test test_name
```

---

## Rule #3: USE PROPER TOOLS

| Task | Use This | NEVER Use |
|------|----------|-----------|
| Find symbols | `grep_search(pattern="...")` | manual scrolling |
| Read code | `read_file` tool | cat/head/tail |
| Edit code | `replace` / `write_file` | partial edits |
| Understand function | Read the code | assumptions |

---

## Rule #4: CITE YOUR SOURCES

Before making changes, cite EXACTLY what you read:

```
I read /home/feanor/Projects/project/src/file.rs:123-456
The function `do_thing` takes parameters X, Y, Z
Therefore I will change...
```

---

## Rule #5: NO DIRTY FIXES

- ❌ "TODO: fix later"
- ❌ `#[allow(dead_code)]` to silence warnings
- ❌ Commenting out broken code
- ❌ Minimal/half-hearted fixes
- ❌ Mocks, stubs, placeholders in production

**ONLY**: Complete, tested, documented code.

---

## Session Start Checklist

When starting work on a project:

1. [ ] Read the project's CLAUDE.md
2. [ ] Read relevant source files
3. [ ] Run `cargo check` or equivalent
4. [ ] TDD: write failing test first
5. [ ] Implement fix/feature
6. [ ] Prove it passes with full output
7. [ ] Update documentation

---

## Code Quality Standards

- Max 300 LOC per file (600 with justification)
- No `unwrap()` in prod paths
- Proper error handling
- No state artifacts in src/

---

## When In Doubt

1. Read the source code
2. Run tests
3. Ask for clarification

**DO NOT GUESS.**
