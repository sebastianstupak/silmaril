---
name: review-pr
description: Review pull request against project standards
trigger: /review-pr
---

# Pull Request Review

Reviews a GitHub pull request against silmaril coding standards and best practices.

## Instructions

1. **Fetch Pull Request**
   ```bash
   # Get PR details by number
   gh pr view <PR_NUMBER>

   # Get PR diff
   gh pr diff <PR_NUMBER>

   # Get PR files
   gh pr view <PR_NUMBER> --json files -q '.files[].path'
   ```

2. **Review Checklist Against CLAUDE.md**
   Check for violations of critical rules:

   ### Code Quality
   - ☐ No println!/eprintln!/dbg! (must use tracing)
   - ☐ Custom error types (no anyhow/Box<dyn Error>)
   - ☐ Platform code abstracted (no #[cfg] in business logic)
   - ☐ Documented public APIs (rustdoc with examples)
   - ☐ Client/server split using macros
   - ☐ Follows coding standards (docs/rules/coding-standards.md)

   ### Testing
   - ☐ Unit tests included
   - ☐ Integration tests if needed
   - ☐ Property-based tests for serialization/math
   - ☐ Tests pass on all platforms

   ### Documentation
   - ☐ Public APIs documented
   - ☐ Code comments for complex logic
   - ☐ Updates to docs/ if changing architecture
   - ☐ Updates to ROADMAP.md if completing tasks

3. **Check File Structure**
   - Files in correct locations
   - No files > 1000 lines in lib.rs
   - Proper module organization
   - No circular dependencies

4. **Review Specific Issues**

   ### Common Anti-Patterns
   - Using print statements instead of tracing
   - Generic error types (anyhow, Box<dyn Error>)
   - Platform-specific #[cfg] in business logic
   - Missing tests
   - Undocumented public APIs
   - Large monolithic files
   - Unsafe code without justification

   ### Performance Concerns
   - Unnecessary allocations in hot paths
   - Missing benchmarks for performance-critical code
   - Inefficient algorithms
   - Not meeting performance targets from docs/performance-targets.md

   ### Security Issues
   - Unsafe code without clear justification
   - Potential panics in production code
   - Missing input validation
   - Anti-cheat bypasses (server-side)

5. **Generate Review Report**
   Create structured feedback with:
   - Summary (approve/request changes/comment)
   - Positive aspects
   - Issues found (critical/major/minor)
   - Suggestions for improvement
   - Code examples where helpful

6. **Post Review** (optional)
   If requested:
   ```bash
   # Post review comment
   gh pr review <PR_NUMBER> --comment -b "Review feedback..."

   # Request changes
   gh pr review <PR_NUMBER> --request-changes -b "Issues found..."

   # Approve
   gh pr review <PR_NUMBER> --approve -b "LGTM!"
   ```

## Output Format

```
Pull Request Review
===================

PR #123: feat(ecs): add sparse set component storage
Author: @username
Files Changed: 8 (+456 -23)
Branch: feature/sparse-set-storage

Summary: REQUEST CHANGES
Overall: Good implementation, but needs fixes before merging

Positive Aspects:
-----------------
✓ Excellent test coverage (98%)
✓ Well-documented public API
✓ Performance benchmarks included
✓ Follows ECS architecture from docs/ecs.md
✓ Proper error handling with custom types

Issues Found:
-------------

CRITICAL (must fix before merge):
❌ Using println! in src/storage.rs:145
   Replace with: tracing::debug!()
   See: CLAUDE.md section "No Printing"

❌ Platform-specific code in business logic (src/lib.rs:234)
   Move to platform abstraction layer
   See: docs/platform-abstraction.md

MAJOR (should fix):
⚠️  Missing integration tests for sparse set operations
   Add tests in tests/integration/ecs.rs
   See: docs/testing-strategy.md

⚠️  Public API `ComponentStorage::get_raw()` is undocumented
   Add rustdoc with examples
   Requirement: CLAUDE.md "Document public APIs"

MINOR (nice to have):
💡 Consider adding more property-based tests for edge cases
💡 Could optimize allocation in hot path (storage.rs:178)

Suggestions:
------------

1. Replace print statements:

   ```rust
   // Before
   println!("Adding component: {}", type_name::<T>());

   // After
   use tracing::debug;
   debug!(
       component_type = type_name::<T>(),
       "Adding component to entity"
   );
   ```

2. Abstract platform code:

   ```rust
   // Before
   #[cfg(windows)]
   fn allocate() { /* windows code */ }

   // After - create platform/allocator.rs with trait
   pub trait Allocator {
       fn allocate(&self, size: usize) -> *mut u8;
   }
   ```

3. Add integration test:

   ```rust
   #[test]
   fn test_sparse_set_add_remove_consistency() {
       let mut storage = SparseSetStorage::new();
       // Test add/remove operations...
   }
   ```

Performance Review:
-------------------
✓ Meets target: spawn_entity < 1μs (actual: 543ns)
✓ Meets target: query < 500ns (actual: 234ns)
✓ Benchmark results included

Files Review:
-------------
✓ engine/core/src/storage.rs        (+234 -12)  GOOD
✓ engine/core/src/lib.rs             (+45 -8)   GOOD
❌ engine/core/src/sparse_set.rs     (+123 -0)  NEEDS FIXES
✓ engine/core/benches/ecs.rs         (+54 -3)   GOOD
✓ docs/ecs.md                        (+28 -5)   GOOD

Compliance Checklist:
---------------------
☐  No print statements
✓  Custom error types
☐  Platform abstraction
✓  Public API documented (except get_raw)
✓  Tests included
✓  Follows coding standards
✓  Performance benchmarks
✓  Updates ROADMAP.md

Next Steps:
-----------
1. Fix critical issues (print statements, platform code)
2. Add missing documentation to get_raw()
3. Add integration tests
4. Re-request review

Once fixed, this will be ready to merge. Good work overall!
```

## Review Guidelines

### When to APPROVE
- All critical issues resolved
- Tests pass
- Documentation complete
- Follows all CLAUDE.md rules
- Performance targets met

### When to REQUEST CHANGES
- Critical issues found
- Missing tests
- Violates coding standards
- Platform abstraction broken
- Performance regressions

### When to COMMENT
- Minor issues only
- Suggestions for improvement
- Questions for clarification
- Positive feedback

## Notes

- Always reference specific files and line numbers
- Provide code examples for suggested fixes
- Be constructive and helpful
- Check against CLAUDE.md rules
- Verify tests exist and pass
- Check performance if applicable
- Reference relevant documentation
- Consider phase context from ROADMAP.md
