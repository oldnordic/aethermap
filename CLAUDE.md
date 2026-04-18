# Development Rules - NON-NEGOTIABLE

**Last Updated:** 2026-01-08
**Status**: MANDATORY for ALL code changes

---

## Rule #1: NEVER GUESS - ALWAYS VERIFY

Before ANY code change:

```bash
# 1. READ the source code first
Read /path/to/file.rs

# 2. CHECK the actual database schema
sqlite3 path/to.db ".schema"

# 3. VERIFY with compiler/tests
cargo test
cargo check
```

**FORBIDDEN**:
- ❌ Assuming what a function does without reading it
- ❌ Guessing database table/column names
- ❌ Writing code based on "how it should work"
- ❌ Using grep to "find" symbols (use `find_symbols` instead)

---

## Rule #2: STORE ARCHITECTURAL DECISIONS

Before ANY code change, document:

### For Bugfixes:
```sql
INSERT INTO architectural_decisions (
    project, decision, reasoning, alternatives, trade_offs,
    session_id, created_at_epoch, created_at
) VALUES (
    'project-name',
    'Fix: [brief description]',
    'Root cause: [PROVEN with evidence]. Fix addresses [specific issue].',
    'Alternative approaches considered...',
    'Risk: [what could break]. Mitigation: [how we prevent regression].',
    'session-id',
    strftime('%s', 'now'),
    datetime('now')
);
```

### For Features/Refactoring:
```sql
INSERT INTO architectural_decisions (
    project, decision, reasoning, alternatives, trade_offs,
    session_id, created_at_epoch, created_at
) VALUES (
    'project-name',
    'Add: [feature name]',
    'Why: [business/technical reason]. Approach: [specific implementation].',
    'Alternative 1: [description] Alternative 2: [description]',
    'Trade-off 1: [cost] Trade-off 2: [complexity]',
    'session-id',
    strftime('%s', 'now'),
    datetime('now')
);
```

---

## Rule #3: TDD - PROVE IT FIRST

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

## Rule #4: USE PROPER TOOLS

| Task | Use This | NEVER Use |
|------|----------|-----------|
| Find symbols | `find_symbols(query="...")` | grep/rg |
| Read code | `Read` tool | cat/head/tail |
| Edit code | `refactor_rename` / `refactor_delete` | manual Edit |
| Understand function | `discover_summary(symbol="...")` | assumptions |
| Check schema | `sqlite3 .db ".schema"` | guessing |
| Check row counts | `sqlite3 .db "SELECT COUNT(*)..."` | assumptions |

---

## Rule #5: CITE YOUR SOURCES

Before making changes, cite EXACTLY what you read:

```
I read /home/feanor/Projects/project/src/file.rs:123-456
The function `do_thing` takes parameters X, Y, Z
I checked .codemcp/operations.db schema
Table `reinjection_briefs` has columns: project, session_id, current_task, ...
Therefore I will change...
```

---

## Rule #6: NO DIRTY FIXES

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
2. [ ] Check docs/DATABASE_SCHEMA.md for schema
3. [ ] Read relevant source files
4. [ ] Run `cargo check` or equivalent
5. [ ] Store architectural decision before coding
6. [ ] TDD: write failing test first
7. [ ] Implement fix/feature
8. [ ] Prove it passes with full output
9. [ ] Update documentation

---

## Code Quality Standards

- Max 300 LOC per file (600 with justification)
- No `unwrap()` in prod paths
- Proper error handling
- No state artifacts in src/
- Modules map to docs/

---

## When In Doubt

1. Read the source code
2. Check the database schema
3. Run tests
4. Store a decision
5. Ask for clarification

**DO NOT GUESS.**
