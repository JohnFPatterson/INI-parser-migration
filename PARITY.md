# Parity report (inih)

## Gate

```
parity-gate: /workspace: PASS: 169 fixtures, 169 compared: 169 identical, 0 logged exceptions, 0 diverged; 0 gate problems.
```

Reproduce:

```sh
make parity
# or
printf '%s' '{"status":"completed","loop_count":0,"workspace_roots":["'"$PWD"'"]}' \
  | ./.cursor/hooks/c-rust-parity/parity_gate.py --force
```

## Oracle shape

Stream parse + handler dump (see `tools/DRIVER_FORMAT.md`). Profiles cover the full `unittest.sh` matrix.

## Per-input results

All 169 fixtures under `tests/parity/**` are **identical** (stdout + exit status) between `./build/oracle` and `./target/release/inih-rust-driver`.

## Quirks matched (C `file:line`)

- `ini_rstrip` end-pointer walk — `ini.c:47-51`
- Inline comments require preceding whitespace — `ini.c:65-73`
- Multiline continuation: non-blank with leading whitespace after prior name — `ini.c:189-200`
- Silent truncation of section/name via `ini_strncpy0` — `ini.c:84-91`, `207`, `236`
- Overlong line “abyss” discard sets error line — `ini.c:165-174`
- UTF-8 BOM skip on first line — `ini.c:177-182`
- `name:value` as well as `name=value` — `ini.c:223-224`
- Handler error on `user=parse_error` (driver policy, same as unittest)

## Divergences

None.

## AddressSanitizer

Built with `gcc -fsanitize=address,undefined` (`make asan-oracles`). Ran oracles over `tests/parity/multi/*`, `heap_realloc/*`, and `string/*` with `ASAN_OPTIONS=detect_leaks=0`. **Clean** (no ASan/UBSan reports).

## Hook-trace

C `build/hook-trace-c` vs Rust `inih-core` example `hook_trace`: **identical**.

## Known gaps / not covered

- C++ `INIReader` out of scope
- AFL fuzzing not required for this port
- Parity gate compares driver stdout/exit only; pointer-level ABI covered in `inih-ffi/tests/abi.rs`
