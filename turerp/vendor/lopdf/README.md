# lopdf (vendored backport)

This is a vendored copy of **lopdf 0.39.0** with the **RUSTSEC-2026-0187** fix
backported from lopdf 0.42. It is consumed by `turerp` via a
`[patch.crates-io]` path dependency — see `Cargo.toml`.

## Why vendored

`printpdf 0.9.1` pins `lopdf = "^0.39.0"` and no published `printpdf` release
(crates.io or git master) consumes `lopdf >= 0.40`. The only fixed version is
`lopdf 0.42`, which is incompatible with `printpdf 0.9.1`. Vendoring 0.39 and
applying the 0.42 recursion-depth bound is the only way to resolve
RUSTSEC-2026-0187 without forking `printpdf`.

## What changed vs upstream 0.39.0

Only two files differ from the published `lopdf 0.39.0`:

- `src/reader.rs` — adds `pub const MAX_NESTING_DEPTH: usize = 100;`
- `src/parser/mod.rs` — threads a `depth: usize` counter through
  `array`, `_dictionary`, `inner_dictionary`, `_direct_objects`, and
  `_direct_object` (mirroring lopdf 0.42), returning
  `nom::Err::Failure(ErrorKind::TooLarge)` when the bound is hit.

The public API (`dictionary`, `direct_object`, `Document`, `Object`, …) is
unchanged, so `printpdf 0.9.1` compiles unmodified. The recursion bound
prevents a crafted PDF with deeply nested arrays/dictionaries from
overflowing the parser stack (a DoS `catch_unwind` cannot catch).

Threat-model note: `turerp` only *generates* PDFs (e-fatura / e-defter) via
`printpdf`; it does not parse untrusted PDF input. The advisory's parse-time
stack overflow is therefore not reachable through `turerp`'s own surface, but
the dependency is still flagged by `cargo audit`, so the backport removes the
advisory at the source.