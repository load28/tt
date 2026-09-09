# Practical diagnostic UI tests

These projects combine independent errors the way an application does. Each
entry source declares every expected primary error on its source line:

```text
broken(); //~ ERROR[code] stable message fragment
```

The runner removes these annotations before compilation. `manifest.json`
holds the structured CLI/LSP contract, while `expected.stderr` stores the
complete normalized CLI presentation. This redundancy is intentional: source
annotations make missing and additional errors obvious, and the baseline makes
formatting, spans, labels, and help text reviewable as one artifact.

After an intentional diagnostic change, regenerate baselines and inspect the
diff before accepting it:

```sh
UPDATE_EXPECT=1 cargo test --test practical_diagnostics
git diff -- tests/fixtures/practical-diagnostics
```

Editor quick fixes with a `fixed` manifest field are applied to the whole
document and compared with that file. The edited document is then republished
to verify that the targeted diagnostic disappears.
