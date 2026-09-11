# SWC parser dependency patch

Source: crates.io `swc_ecma_parser` 45.0.0, upstream commit
`9170cd59ebf735925c00cf0fbe615d1feb181431`, directory `crates/swc_ecma_parser`.
The dependency is Apache-2.0 licensed; see LICENSE.

Local change: `src/lexer/mod.rs`, `read_jsx_entity`'s numeric conversion
returns an optional code point and preserves unrecognized references as text.
Empty digits, arithmetic overflow, and code points above U+10FFFF previously
returned None and were unwrapped. Both hexadecimal and decimal references now
use the same literal representation as unknown named references. TypeScript
accepts these sources; inventing a syntax error would break compatibility.

The entity scanner stops at non-entity characters before
advancing, preserving JSX delimiters and UTF-8 text for the enclosing scanner.

`tests/jsx_entities.rs` in the parent repository tests the dependency directly.
The direct path dependency also applies when ttc is built by the standalone
fuzz workspace. Remove this vendored copy only after an upstream version
passes these regressions without the patch. This copy retains upstream source,
including upstream unsafe blocks; the patch introduces no unsafe code.
