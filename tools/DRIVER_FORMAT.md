# Differential driver format (inih)

Both `./build/oracle` (C) and `./target/release/inih-rust-driver` (Rust) print the same bytes for the same fixture path. Exit status is always `0` unless the driver itself fails to run (I/O). Parse errors are reported in the output (`e=`), not via exit status.

## Invocation

```
./build/oracle <fixture>
./target/release/inih-rust-driver <fixture>
```

`<fixture>` is a path relative to the repo root under `tests/parity/<profile>/…`.

Optional: `--sections <name>[,<name>…]` — currently only `parse` is defined; with or without `--sections`, output is identical (single section).

Optional: `--profile <name>` — overrides profile inferred from the fixture path (`tests/parity/<profile>/…`).

## Profile → compile-time / config

| Profile | Mode | Key `INI_*` / config |
|---------|------|----------------------|
| `multi` | file | defaults (multiline on, stack, max_line 200) |
| `multi_max_line` | file | `INI_MAX_LINE=20` |
| `single` | file | `INI_ALLOW_MULTILINE=0` |
| `disallow_inline_comments` | file | `INI_ALLOW_INLINE_COMMENTS=0` |
| `stop_on_first_error` | file | `INI_STOP_ON_FIRST_ERROR=1` |
| `handler_lineno` | file | `INI_HANDLER_LINENO=1` |
| `heap` | file | `INI_USE_STACK=0` |
| `heap_max_line` | file | `INI_USE_STACK=0`, `INI_MAX_LINE=20`, `INI_INITIAL_ALLOC=20` |
| `heap_realloc` | file | `INI_USE_STACK=0`, `INI_ALLOW_REALLOC=1`, `INI_INITIAL_ALLOC=5` |
| `heap_realloc_max_line` | file | `INI_USE_STACK=0`, `INI_MAX_LINE=20`, `INI_ALLOW_REALLOC=1`, `INI_INITIAL_ALLOC=5` |
| `call_handler_on_new_section` | file | `INI_CALL_HANDLER_ON_NEW_SECTION=1` |
| `allow_no_value` | file | `INI_ALLOW_NO_VALUE=1` |
| `string` | string | `INI_MAX_LINE=20` (file contents fed to `ini_parse_string`) |
| `heap_string` | string | `INI_USE_STACK=0`, `INI_MAX_LINE=20`, `INI_INITIAL_ALLOC=20` |
| `alloc` | string+alloc | `INI_CUSTOM_ALLOCATOR=1`, `INI_USE_STACK=0`, `INI_ALLOW_REALLOC=1`, `INI_INITIAL_ALLOC=12` |

## `no_file.ini`

When the fixture basename is `no_file.ini`, drivers call `ini_parse` on a path that does not exist (`__inih_no_such_file__.ini`). Fixture file contents are ignored.

## Handler dump lines

Global `User` starts at `0`. Before parse, `user` pointer targets an `int` with value `100`. Each handler invocation sets `User = *user`.

When the section changes, or `name` is NULL (new-section callback):

```
... [<section>]
```

For each name (and optional value):

Without lineno:

```
... <name>=<value>;
```

or if `value` is NULL (`allow_no_value`):

```
... <name>;
```

With `handler_lineno`:

```
... <name>=<value>;  line <lineno>
```

or no-value:

```
... <name>;  line <lineno>
```

Handler returns `0` (error) when `name=="user"` and `value=="parse_error"`; otherwise `1`.

## Summary line

After parse (file mode), basename of the intended name:

```
<basename>: e=<code> user=<User>
```

String mode (`string`, `heap_string`):

```
<stem>: e=<code> user=<User>
```

where `<stem>` is the fixture basename without `.ini` (`empty_string`, `basic`, …). For `empty_string` the display name is `empty string` (space). Mapping:

| fixture stem | printed name |
|--------------|--------------|
| `empty_string` | `empty string` |
| `long_line` | `long line` |
| `long_continued` | `long continued` |
| other | stem as-is |

Alloc mode (`alloc`):

Custom allocator hooks print before/during/after parse:

```
ini_malloc(<size>)
ini_realloc(<size>)
ini_free()
```

Dumper lines as above (no `user=` in summary). Summary:

```
<stem>: e=<code>
```

## Exit status

`0` on successful driver execution. Nonzero only for driver usage errors (missing args, unknown profile).
