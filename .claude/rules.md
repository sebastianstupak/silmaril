# Claude Code Rules for Silmaril

## Task Completion & Documentation Rules

### No Task Summaries in Repository
**CRITICAL:** Task completion summaries, implementation notes, and temporary documentation MUST NOT be created in the repository.

**Instead:**
- Write all task summaries to the scratchpad directory: `C:\Users\sebas\AppData\Local\Temp\claude\D--dev-silmaril\a68515e4-2a8b-4e96-b0c4-1cc05c58bbbd\scratchpad\`
- Create a `task-completions/` subdirectory for completion summaries
- Reference these files from there when needed

**DO NOT CREATE:**
- `*_COMPLETE.md` files in repository root
- `TASK_*.md` files anywhere in the repository
- `*_INTEGRATION_COMPLETE.md` files
- Any temporary `.backup`, `.patch`, or debugging output files in the repository

**Exception:**
- Permanent documentation in `docs/` that becomes part of the project knowledge base
- Official README, ROADMAP, CLAUDE.md, and other project-level documentation

---

## Code Organization Rules

### No Example Folders in Engine Modules
**IMPORTANT:** Do NOT create `examples/` folders in engine modules (`engine/*`).

Examples should be placed in one of these locations instead:
- **Tests**: If the example demonstrates correctness or validates behavior → `tests/`
- **Benchmarks**: If the example measures performance → `benches/`
- **Integration Tests**: If the example shows module integration → `tests/integration_*.rs`

#### Rationale
- Examples are not built in release mode and don't get tested in CI
- Tests and benchmarks are automatically validated and maintained
- Clear separation between demonstration code and validated code
- Better organization and discoverability

#### Exception
The root-level `examples/` directory for full application examples is acceptable.

---

## Module Structure Rules

All engine modules should follow this structure:
```
engine/<module>/
├── src/           # Source code
├── tests/         # Unit and integration tests
├── benches/       # Performance benchmarks
├── build.rs       # Build-time checks (uses engine-build-utils)
├── Cargo.toml     # Module configuration
└── CLAUDE.md      # Module documentation for Claude
```

**Do NOT include:**
- `examples/` directory in engine modules (use tests/ or benches/ instead)
- Debugging print statements (println!, dbg!, eprintln!) outside of tests
- Error types without the `define_error!` macro

---

## Build-time Enforcement

All engine modules use `engine-build-utils` for architectural enforcement:
- **Print Check**: Fails build if println!/dbg!/eprintln! found in src/
- **Error Check**: Ensures all error types use `define_error!` macro
- **Module Check**: Validates module structure compliance

See `engine/build-utils/` for implementation details.
