import * as assert from "node:assert/strict";
import { test } from "node:test";
import { Diagnostic, DiagnosticSeverity, DiagnosticTag } from "vscode-languageserver/node";

import { publishedDiagnostics } from "../diagnostics";
import { TextDocument } from "vscode-languageserver-textdocument";
import { toDiagnostic } from "../lsp-projections";

test("compiler missing-token ranges stay zero-width before Unicode and at EOF", () => {
  const doc = TextDocument.create("file:///missing.tt", "tt", 1, "value.😀\r\n");
  for (const [line, col] of [[1, 7], [2, 1]]) {
    const diagnostic = toDiagnostic(doc, { line, col, endLine: line, endCol: col, code: "ts1003", message: "Identifier expected." }, file => file);
    assert.deepEqual(diagnostic.range.start, diagnostic.range.end);
  }
});

const at = (line: number, character: number, length: number) => ({
  start: { line, character },
  end: { line, character: character + length },
});

const diagnostic = (
  source: string,
  code: string | number,
  line: number,
  character: number,
  length: number,
  severity: DiagnosticSeverity = DiagnosticSeverity.Error,
): Diagnostic => ({
  range: at(line, character, length),
  severity,
  code,
  source,
  message: `${source} ${code}`,
});

const shown = (list: Diagnostic[]) => list.map((d) => `${d.source} ${d.code}`);

test("two rules the typed pass reports at one call are both published", () => {
  const published = publishedDiagnostics({
    text: [],
    service: [diagnostic("ts", 2322, 2, 2, 6), diagnostic("ts", 2554, 2, 9, 5)],
    restates: [],
    hints: [],
    typed: {
      replacesTypes: true,
      diagnostics: [diagnostic("ttc", "ts2554", 2, 9, 5), diagnostic("ttc", "ts2322", 2, 9, 7)],
    },
  });
  assert.deepEqual(shown(published), ["ttc ts2554", "ttc ts2322"]);
});

test("the typed pass replaces another layer's statement of the same rule at the same start", () => {
  const published = publishedDiagnostics({
    text: [diagnostic("ttc", "match-not-exhaustive", 4, 9, 9)],
    service: [diagnostic("ts", 2322, 6, 2, 3)],
    restates: [],
    hints: [],
    typed: {
      replacesTypes: false,
      diagnostics: [
        { ...diagnostic("ttc", "match-not-exhaustive", 4, 9, 9), message: "typed" },
        diagnostic("ttc", "ts2322", 6, 2, 3),
      ],
    },
  });
  assert.deepEqual(shown(published), ["ttc match-not-exhaustive", "ttc ts2322"]);
  assert.equal(published[0].message, "typed");
});

test("a different rule at the same start as another layer's is its own diagnostic", () => {
  const published = publishedDiagnostics({
    text: [diagnostic("ttc", "missing-arm-body", 5, 4, 11)],
    service: [],
    restates: [],
    hints: [],
    typed: { replacesTypes: true, diagnostics: [diagnostic("ttc", "ts2304", 5, 4, 2)] },
  });
  assert.deepEqual(shown(published), ["ttc ts2304", "ttc missing-arm-body"]);
});

test("a suggestion at the start of a typed diagnostic is kept beside it", () => {
  const unused = {
    ...diagnostic("ts", 6133, 1, 6, 5, DiagnosticSeverity.Hint),
    tags: [DiagnosticTag.Unnecessary],
  };
  const published = publishedDiagnostics({
    text: [],
    service: [unused],
    restates: [],
    hints: [],
    typed: { replacesTypes: true, diagnostics: [diagnostic("ttc", "ts6133", 1, 6, 5)] },
  });
  assert.deepEqual(shown(published), ["ts 6133", "ttc ts6133"]);
});

test("a compiler diagnostic the service restates is left to the service", () => {
  const published = publishedDiagnostics({
    text: [diagnostic("ttc", "source-not-typescript", 9, 12, 1)],
    service: [diagnostic("ts", 1109, 9, 12, 1)],
    restates: ["source-not-typescript"],
    hints: [],
    typed: { replacesTypes: false, diagnostics: [diagnostic("ttc", "source-not-typescript", 9, 12, 1)] },
  });
  assert.deepEqual(shown(published), ["ts 1109"]);
});

test("a repaired primary survives another raw syntax error with the same code", () => {
  const layers = {
    text: [diagnostic("ttc", "source-not-typescript", 0, 12, 1), diagnostic("ttc", "source-not-typescript", 2, 0, 0)],
    service: [diagnostic("ts", 1109, 0, 12, 1)],
    restates: ["source-not-typescript"],
    retains: [{ code: "source-not-typescript", start: { line: 2, character: 0 } }],
    hints: [], typed: null,
  };
  assert.deepEqual(shown(publishedDiagnostics(layers)), ["ts 1109", "ttc source-not-typescript"]);
});
