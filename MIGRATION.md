# Migration: inih C → Rust

## Oracle shape

SAX-style **stream parse + callback dump**. Drivers invoke `ini_parse` / `ini_parse_string` (per profile) and print handler calls plus the parse return code. See [`tools/DRIVER_FORMAT.md`](tools/DRIVER_FORMAT.md).

## Layout

| Path | Role |
|------|------|
| `ini.c` / `ini.h` | Unmodified C oracle |
| `inih-core/` | All parser logic; `#![forbid(unsafe_code)]` |
| `inih-ffi/` | Thin C ABI (`ini_parse*`); only crate with `unsafe` |
| `inih-rust-driver/` | Differential driver (byte-identical to C oracle) |
| `tools/inih-oracle.c` | C oracle driver (public API only) |
| `tests/parity/<profile>/` | Fixtures for the full `INI_*` matrix |

## Out of scope

- `cpp/INIReader` and C++ examples
- AFL fuzzing under `fuzzing/`
- Changing legacy C to clear Sonar findings

## Feature matrix

`IniConfig` in `inih-core` mirrors every meson / `INI_*` toggle used by `tests/unittest.sh`. Profiles: `multi`, `multi_max_line`, `single`, `disallow_inline_comments`, `stop_on_first_error`, `handler_lineno`, `heap`, `heap_max_line`, `heap_realloc`, `heap_realloc_max_line`, `call_handler_on_new_section`, `allow_no_value`, `string`, `heap_string`, `alloc`.

`inih-ffi` Cargo features map to the same toggles for ABI builds (`multi-line`, `utf8-bom`, `inline-comments`, `use-stack` / `use-heap`, `allow-realloc`, `stop-on-first-error`, `handler-lineno`, `call-handler-on-new-section`, `allow-no-value`, `custom-allocator`).

## Unsafe audit

```
grep -rn "unsafe" --include='*.rs' --exclude-dir=target
```

- `inih-core`: only `forbid(unsafe_code)`.
- `inih-ffi`: `unsafe extern "C"` exports and blocks, each with `SAFETY:`.
- Driver / tests: no library `unsafe` outside FFI ABI tests calling C exports.

## Export check

Header pattern: `INI_API int <name>(` / prototypes in `ini.h`. Expected symbols: `ini_parse`, `ini_parse_file`, `ini_parse_stream`, `ini_parse_string`, `ini_parse_string_length`. Verified via `make export-check`.

## Behavior changed on purpose (approved)

| ID | Change | Reason | Approval | Pinning test |
|----|--------|--------|----------|--------------|
| CH-001 | Demo intentional change with no pinning test | Bugbot Rule 4 demo |  |  |

DEMO: Bugbot Rule 4 — `CH-001` has empty Approval and Pinning test columns.

## SonarQube findings summary

Stored analyses (cloud-hosted MCP; no new scan) for the inih / INI-parser-migration
Sonar projects on branch `master` (analyses 2026-10-01 and 2026-10-02). Project keys are not recorded in-repo.

| Rule | C behavior | Rust belief |
|------|------------|-------------|
| `c:S912` @ `ini.c:49` (BLOCKER, maintainability) — side effect in RHS of `&&` inside `ini_rstrip` | Walks backward with `*--end` while testing `isspace` | Port uses an equivalent rstrip loop without `&&` side effects; **driver-visible bytes still match C**. Not treated as a behavior-changing fix; not a `PE-NNN`. |
| `githubactions:S7637` @ `.github/workflows/tests.yml` | CI pin style | Out of scope for the library port |

No `TO_REVIEW` security hotspots on library C. Analysis does not include the new Rust crates (belief).

**CI / Sonar policy:** A quality gate must not fail the port solely on legacy C ratings. Do not patch `ini.c` / `ini.h` to clear findings.

## Hook-trace

`tools/hook-trace.c` (C, custom allocator + handler) vs `inih-core` example `hook_trace` — **identical** on the fixed sample (`M`/`R`/`F`/`H`/`RESULT`). FFI custom-allocator feature is reserved for linking caller `ini_malloc`/`ini_free`/`ini_realloc`; allocation observability for the matrix is covered by core `HeapHooks` (alloc profile parity).

## Tests

| Suite | Against |
|-------|---------|
| `inih-core/tests/ported_unittest.rs` | Ported cases from `tests/unittest*.c` |
| `inih-core/tests/exceptions.rs` | PE pinning (none) |
| `inih-ffi/tests/abi.rs` | Public FFI symbols |
| Existing `tests/unittest.sh` | Still runnable against original C |

Removed from FFI white-box: none (C suite is public-API / dump based; static helpers covered under `inih_core::internals`).

## One-command demo

```sh
make parity
```
