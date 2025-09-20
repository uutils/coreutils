# stty save/restore behavior in uutils

This page documents the `-g` (save) format and round-trip behavior of `uutils stty`.

## `-g` output format

`uutils stty -g` prints the current terminal settings in a colon-separated hexadecimal format that is designed to be read back by `uutils stty` itself.

The format is:

```
<input_flags_hex>:<output_flags_hex>:<control_flags_hex>:<local_flags_hex>:<cc0>:<cc1>:...:<ccN>
```

- The first four fields are the termios flag bitfields (input, output, control, local), printed as hexadecimal numbers.
- The remaining fields are the control character bytes (CCs), each printed as a 1–2 digit hexadecimal value. The number of CC fields depends on the platform (the platform’s NCCS value).

Example:

```
6b02:3:4b00:200005cf:4:ff:ff:7f:17:15:12:0:3:1c:1a:19:11:13:16:f:1:0:14:0
```

## Round-trip compatibility

`uutils stty` supports reading back its own `-g` output:

```
stty "$(stty -g)"
```

This restores the same settings and exits with code 0. Unknown/unsupported flag bits on a given platform are safely ignored.

## GNU `stty` compatibility (gfmt1)

GNU `stty -g` commonly prints a `gfmt1` key/value format, e.g.:

```
gfmt1:cflag=4b00:iflag=6b02:lflag=200005cf:oflag=3:...:ispeed=38400:ospeed=38400
```

Currently, `uutils stty` does not parse `gfmt1`. Use `uutils stty -g` output for restore with `uutils stty`.

Mixing formats between `uutils stty` and GNU `stty` may fail. If you must interoperate, prefer using the same implementation for both save and restore steps.

## Platform notes

- The number of control characters (NCCS) and the underlying bit width for termios flags vary by platform. `uutils stty` reads NCCS at runtime and truncates unknown bits when applying flags.
- Hexadecimal is case-insensitive. Empty CC fields are treated as 0.

## Error handling

- Malformed hex values or a mismatched number of CC fields result in a non-zero exit code and an error message (e.g., "invalid argument" or "invalid integer argument").

## Future compatibility

Support for reading GNU `gfmt1` may be considered in future versions. For now, rely on `uutils stty`’s colon-separated hex format for save/restore round-trips.



## Advanced edge-case validation

The implementation has been validated with an extensive suite of edge-case tests to ensure robustness across platforms and terminals. Highlights:

- Boundary conditions
  - Minimal valid payload: accept a string with the four flag fields plus exactly NCCS control characters, all set to 0; succeeds and normalizes on reprint.
  - Case-insensitivity: accept uppercase and mixed-case hex for all fields; round-trip preserves state.
  - Leading zeros: accept arbitrarily padded fields; output normalizes to minimal-width hex.

- Error handling
  - Insufficient fields (< 5 total): rejected with exit code 1 and "invalid argument".
  - Extra CC fields (> NCCS): rejected with exit code 1 and "invalid argument".
  - Malformed hex in any flag field: rejected with exit code 1 and "invalid integer argument '&lt;chunk&gt;'".
  - Unexpected characters (spaces, punctuation): rejected early with exit code 1 and "invalid integer argument '&lt;input&gt;'".

- Platform compatibility
  - Exact CC count enforced using the platform’s runtime NCCS; NCCS−1 and NCCS+1 inputs are rejected.
  - Flag fields parsed into platform tcflag_t width; unknown bits are safely ignored (truncate semantics).

- Data integrity
  - Unknown/high bits in flags are accepted but do not persist when re-saved; round-tripping returns to canonical values.

- Security considerations
  - Oversized inputs (e.g., thousands of CC entries) are rejected quickly via count validation; no excessive CPU or memory use observed.

These tests live under tests/by-util/test_stty_roundtrip.rs and tests/by-util/test_stty_hex_edges.rs and run under the feature flag `stty`.

## Troubleshooting and examples

- Restore from current settings
  - stty "$(stty -g)"

- Uppercase input
  - stty "$(stty -g | tr 'a-f' 'A-F')" # succeeds

- Leading zeros
  - Provide any number of leading zeros per field; the next `stty -g` prints normalized hex.

- Insufficient fields
  - stty "6b02:3:4b00:200005cf" # fails with: invalid argument '...'

- Malformed hex
  - stty "6b02:zz:4b00:200005cf:..." # fails with: invalid integer argument 'zz'

- Trailing whitespace or punctuation
  - stty "$(stty -g) " # fails with: invalid integer argument '...'

## PTY and /dev/tty

- Tests prefer using /dev/tty when available; CI also exercises a PTY-backed path so behavior is validated on real terminals and pseudo-terminals.
- If /dev/tty is unavailable, some tests are skipped; the utility itself will error if the selected file descriptor is not a terminal (consistent with termios behavior).

## Cross-platform behavior

- Termios flag widths (tcflag_t) differ (Linux commonly u32; macOS/BSD may be u64). The parser uses tcflag_t and from_bits_truncate to remain portable.
- The number and meaning of control characters differ across platforms; the parser enforces the exact CC count for the current platform.
- ispeed/ospeed are not encoded in the colon-hex format; `uutils stty` does not parse or set speeds from `-g` input. This is documented and by design.

## Performance and safety

- Parsing uses safe Rust conversions and bounded operations; no unsafe code paths in the hex parser.
- Large malformed inputs are rejected by early validation (character filter and CC count), preventing excessive allocations or quadratic behavior.

## CI coverage

- Matrix includes Linux and macOS. Tests cover:
  - Round-trip save/restore
  - Mixed-case hex and leading zeros
  - Error cases (insufficient/extra fields, malformed hex, unexpected characters)
  - NCCS mismatch rejection
  - Unknown flag-bit truncation behavior

