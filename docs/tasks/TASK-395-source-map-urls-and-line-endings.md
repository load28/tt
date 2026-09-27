# TASK-395: Encode source map URLs and end the map comment with the output's line ending

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`--source-map` wrote raw file names where the format requires URLs, so a
file name containing a space, `#`, or `%` produced a map that consumers could
not follow. The appended comment line also always used `\n`, even in output
that uses CRLF.

## Scope

- Included: The URL written in `//# sourceMappingURL=` and in the map's
  `sources` (`src/main/output.rs`), and the line terminator of the appended
  comment (`src/main/build.rs`).
- Excluded: The `file` field (see Decision 2), the mappings themselves, the
  inline `data:` URL (already a valid URL), and the library API in
  `src/source_map.rs`, whose callers supply the names.

## Defects

1. `ttc --source-map file -o out "a b.tt"` wrote
   `//# sourceMappingURL=a b.ts.map` and `"sources":["../a b.tt"]`.
   `node --enable-source-maps` did not apply the map (the URL ends at the
   space), and a `#` or `%` in a name was read as a fragment or an escape.
2. For CRLF input, and for BOM plus CRLF input, the output used CRLF but the
   comment line (and the break inserted before it when the output did not end
   with one) used `\n`, which gave the file mixed line endings.

## Decisions

### Decision 1: Serialize names as relative URL paths

- **Context**: ECMA-426 (Source Map, 1st edition), section "Source map
  format": each `sources` entry is "a (potentially relative) URL", resolved
  against the source map's URL; the `sourceMappingURL` comment carries a URL
  that is resolved against the generated file's URL. The WHATWG URL Standard,
  which ECMA-426 uses for resolution, treats `?` as the start of the query,
  `#` as the start of the fragment, `%` as the start of a percent-encoded
  byte, and `\` as a path separator for special schemes such as `file:`.
- **Alternatives considered**: Encoding every byte outside RFC 3986's
  unreserved set is valid but also rewrites names containing sub-delimiters
  such as `@` or `+` that already worked. Encoding inside
  `SourceMap::to_json` would reinterpret names supplied by library callers.
- **Decision and rationale**: The CLI converts each path segment with the
  WHATWG "path percent-encode set" (C0 controls, space, `"`, `#`, `<`, `>`,
  `?`, `^`, `` ` ``, `{`, `}`, and every byte above `0x7E`) plus `%` and `\`,
  which must be escaped for the name to round-trip, and `:` in the first
  segment. RFC 3986 section 4.2 forbids a colon in the first segment of a
  relative-path reference because it would be parsed as a scheme. Non-ASCII
  bytes are written as their UTF-8 percent encoding, which is what a WHATWG
  URL serializer produces for such a path anyway, so resolution is unchanged.
  Every name that was already a valid URL path is written unchanged.

### Decision 2: Leave `file` as a name

- **Context**: ECMA-426 describes `file` as the name of the generated code and
  does not require it to be a URL.
- **Decision and rationale**: Keep writing the output's file name verbatim,
  so existing consumers that display it are unaffected.

### Decision 3: Terminate the comment with `ttc::line_ending` of the output

- **Context**: The banner already joins the output with
  `ttc::line_ending(&code)`, the repository's rule for generated text in an
  existing file.
- **Decision and rationale**: `source_map_for` receives the output's line
  ending, the break inserted before the comment uses it, and the comment line
  ends with it. `SourceMap::url_comment` still supplies the comment syntax;
  only its terminator is replaced. LF output is byte-for-byte unchanged.

## Work log

- 2026-09-27: Reproduced both defects with the `sm1` and `p4` investigation
  material. Added `source_map_urls_percent_encode_file_names` (asserts the
  comment URL, `sources`, and, when node is available, that
  `node --enable-source-maps` reports `a b#1%.tt:2:`) and
  `source_map_comments_use_the_line_ending_of_the_output` (CRLF, BOM plus
  CRLF, and LF inputs, in `file` and `inline` modes) to
  `tests/cli_outputs.rs`. Both failed against the original code: the comment
  was `a b#1%.ts.map`, and the CRLF output ended with
  `export { a };\n//# sourceMappingURL=crlf.ts.map\n`.
- 2026-09-27: Added `url_path` in `src/main/output.rs`, used it for the map
  file URL and in `relative_to`, and passed the line ending from
  `src/main/build.rs`. Both tests passed. Manually confirmed that a name with
  non-ASCII bytes and a colon (`é:x.tt`) is written as `%C3%A9%3Ax.ts.map` and
  that node still maps the frame to `é:x.tt:2:1`.
- 2026-09-27: Updated the source map bullet in `docs/ai/tt.md`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Changed `src/main/output.rs`, `src/main/build.rs`, `tests/cli_outputs.rs`,
and `docs/ai/tt.md`. Source maps for any file name now resolve in
spec-conforming consumers, and CRLF outputs keep a single line ending.
