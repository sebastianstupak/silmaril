# Phase 0.2: Repository Setup

**Status:** ⚪ Not Started
**Estimated Time:** 2-3 hours
**Priority:** Critical

---

## 🎯 **Objective**

Set up the repository structure, Cargo workspace, and development tooling before writing engine code.

---

## 📋 **Tasks**

### **1. Cargo Workspace**

Create workspace root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "engine/core",
    "engine/renderer",
    "engine/networking",
    "engine/physics",
    "engine/audio",
    "engine/lod",
    "engine/interest",
    "engine/auto-update",
    "engine/observability",
    "engine/macros",
    "engine/binaries/client",
    "engine/binaries/server",
    "engine/dev-tools/hot-reload",
]

[workspace.dependencies]
# Core
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.35", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }

# Math
glam = { version = "0.29", features = ["serde"] }

# Graphics
ash = "0.38"
gpu-allocator = { version = "0.27", features = ["vulkan"] }
winit = "0.30"

# Serialization
bincode = "1.3"
rkyv = "0.7"
flatbuffers = "24.3"

# Testing
proptest = "1.4"
criterion = "0.5"

[profile.dev]
opt-level = 1  # Faster debug builds

[profile.dev.package."*"]
opt-level = 3  # Optimize dependencies even in debug

[profile.release]
lto = "thin"
codegen-units = 1
opt-level = 3

[profile.bench]
inherits = "release"
debug = true  # For profiling
```

---

### **2. Directory Structure**

Create all directories:

```bash
mkdir -p engine/{core,renderer,networking,physics,audio,lod,interest,auto-update,observability,macros}
mkdir -p engine/binaries/{client,server}
mkdir -p engine/dev-tools/{hot-reload,docker}
mkdir -p examples/{singleplayer,mmorpg,turn-based,moba}
mkdir -p docs/{tasks,rules,decisions}
mkdir -p scripts
mkdir -p .github/workflows
```

---

### **3. .gitignore**

Create `.gitignore`:

```gitignore
# Rust
target/
Cargo.lock
**/*.rs.bk
*.pdb

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db
.DS_Store?
._*
.Spotlight-V100
.Trashes

# Build artifacts
*.exe
*.dll
*.so
*.dylib

# Profiling
*.prof
perf.data*
flamegraph.svg

# Assets (too large for git)
assets/**/*.bin
assets/**/*.pak

# Temp
tmp/
temp/
*.tmp

# Env files
.env
.env.local

# Coverage
lcov.info
coverage/
```

---

### **4. .cargo/config.toml**

Enforce lints and standards:

```toml
[target.'cfg(all())']
rustflags = ["-D", "warnings"]  # Deny warnings in CI

[build]
jobs = 4  # Parallel compilation

[lints.rust]
unsafe_code = "forbid"  # No unsafe unless explicitly allowed
missing_docs = "warn"   # Warn on missing public API docs

[lints.clippy]
# Deny these
print_stdout = "deny"
print_stderr = "deny"
dbg_macro = "deny"
todo = "warn"
unwrap_used = "warn"
expect_used = "warn"

# Pedantic
pedantic = "warn"
nursery = "warn"

# Allow some pedantic lints
cast_precision_loss = "allow"
module_name_repetitions = "allow"
```

---

### **5. rustfmt.toml**

Code formatting config:

```toml
edition = "2021"
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Max"
fn_call_width = 80
attr_fn_like_width = 80
struct_lit_width = 80
struct_variant_width = 80
array_width = 80
chain_width = 80
single_line_if_else_max_width = 80
wrap_comments = true
format_code_in_doc_comments = true
normalize_comments = true
normalize_doc_attributes = true
format_strings = true
format_macro_matchers = true
format_macro_bodies = true
hex_literal_case = "Upper"
empty_item_single_line = true
struct_lit_single_line = true
fn_single_line = false
where_single_line = false
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
reorder_imports = true
reorder_modules = true
reorder_impl_items = true
type_punctuation_density = "Wide"
space_before_colon = false
space_after_colon = true
spaces_around_ranges = false
binop_separator = "Front"
remove_nested_parens = true
combine_control_expr = true
overflow_delimited_expr = true
struct_field_align_threshold = 0
enum_discrim_align_threshold = 0
match_arm_blocks = true
match_arm_leading_pipes = "Never"
force_multiline_blocks = false
fn_args_layout = "Tall"
brace_style = "SameLineWhere"
control_brace_style = "AlwaysSameLine"
trailing_semicolon = true
trailing_comma = "Vertical"
use_field_init_shorthand = true
force_explicit_abi = true
condense_wildcard_suffixes = true
color = "Auto"
required_version = "1.7.0"
unstable_features = false
disable_all_formatting = false
skip_children = false
hide_parse_errors = false
error_on_line_overflow = false
error_on_unformatted = false
report_todo = "Always"
report_fixme = "Always"
ignore = []
emit_mode = "Files"
make_backup = false
```

---

### **6. LICENSE**

Apache-2.0 license file:

```
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   [Full Apache 2.0 text - see https://www.apache.org/licenses/LICENSE-2.0.txt]
```

---

### **7. .editorconfig**

For cross-editor consistency:

```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true

[*.rs]
indent_style = space
indent_size = 4
max_line_length = 100

[*.toml]
indent_style = space
indent_size = 2

[*.{yml,yaml}]
indent_style = space
indent_size = 2

[*.md]
trim_trailing_whitespace = false
max_line_length = 0

[Makefile]
indent_style = tab
```

---

## ✅ **Acceptance Criteria**

- [ ] Cargo workspace compiles (even with empty crates)
- [ ] All directories created
- [ ] .gitignore covers all artifacts
- [ ] Lints configured in .cargo/config.toml
- [ ] rustfmt.toml present
- [ ] LICENSE file added
- [ ] Can run `cargo fmt --check` (passes)
- [ ] Can run `cargo clippy` (no warnings)

---

## 🧪 **Verification**

```bash
# Check workspace
cargo check --workspace

# Format check
cargo fmt --check

# Clippy
cargo clippy --workspace -- -D warnings

# Build (should succeed with empty crates)
cargo build --workspace
```

---

**Dependencies:** [phase0-documentation.md](phase0-documentation.md)
**Next:** [phase0-cicd.md](phase0-cicd.md)
