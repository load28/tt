# TASK-384 verification evidence

These are post-repair results for the failure families recorded in TASK-383.
The earlier task's artifacts intentionally retain their baseline findings.

From the repository root, build `target/debug/ttc` and run:

```sh
mkdir -p target/task384
node docs/tasks/evidence/TASK-384/composition.mjs
TTC_BINARY="$PWD/target/debug/ttc" npm test --prefix integrations/unplugin
node docs/tasks/evidence/TASK-383/run-editor.mjs
```

The composition runner removes each disposable source after checking it, avoiding
quadratic project rescans of previously generated cases. Its 72 remaining
rejections are existing `match-placement` diagnostics for unsupported conditional
placements. All other 1,743 combinations compile, with no internal compiler errors.

Production, runtime, bundler and React probes reuse TASK-383's runners. For a new
CLI output run, use a fresh fixture directory by replacing `target/audit-383/`
with `target/task384/` (or another disposable path) in a copy of the runner.
Compiler ownership deliberately refuses baseline outputs that have no ownership
record. No real user project, installed extension, or release artifact is changed.

The editor result uses the repository extension in an isolated actual VS Code
profile, without the separately installed native TypeScript extension. Native
backend checks are covered by the repository native suite and the local CI gate.

The results are macOS arm64 observations. They do not claim exhaustive language
coverage or verification of every bundler adapter and operating system.
