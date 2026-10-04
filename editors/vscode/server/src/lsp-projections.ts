/** LSP answer projection without connection or validation state. */
import {
  CodeAction, CodeActionKind, Diagnostic, DiagnosticSeverity, DocumentSymbol,
  Range, SymbolKind, TextDocumentEdit, TextEdit,
} from "vscode-languageserver/node";
import type { TextDocument } from "vscode-languageserver-textdocument";
import * as analysis from "./analysis";
import type * as engine from "./engine";
import type * as ttc from "./ttc";

export function toDiagnostic(
  doc: TextDocument,
  d: ttc.TtcDiagnostic,
  editorUri: (file: string) => string,
): Diagnostic {
  let range: Range;
  if (d.line > 0) {
    const start = { line: d.line - 1, character: Math.max(0, d.col - 1) };
    const offset = doc.offsetAt(start);
    // The compiler's own range wins: it knows the construct's extent, so
    // the squiggle covers `try parse(text)` or `match (shape)` whole.
    // Without one, the word at the position is the best guess there is.
    const reported =
      d.endLine !== undefined && d.endLine > 0 && d.endCol !== undefined
        ? { line: d.endLine - 1, character: Math.max(0, d.endCol - 1) }
        : null;
    const word = analysis.wordAt(doc.getText(), offset);
    const end =
      reported && doc.offsetAt(reported) > offset
        ? reported
        : word && word.start === offset
          ? doc.positionAt(word.end)
          : { line: start.line, character: start.character + 1 };
    range = { start, end };
  } else {
    // Positionless (output-verification) errors: flag the first line.
    range = {
      start: { line: 0, character: 0 },
      end: { line: 1, character: 0 },
    };
  }
  return {
    severity: DiagnosticSeverity.Error,
    range,
    message: d.message,
    code: d.code,
    source: "ttc",
    // The compiler's secondary labeled spans. Typed diagnostics replace
    // the language-service layer under the default settings, so the
    // related places must travel on this path too or the editor loses
    // them the moment the typed answer lands.
    relatedInformation: d.labels?.length
      ? d.labels.map((label) => ({
          location: {
            uri: label.path ? editorUri(label.path) : doc.uri,
            range: {
              start: {
                line: Math.max(0, label.line - 1),
                character: Math.max(0, label.col - 1),
              },
              end: {
                line: Math.max(0, label.endLine - 1),
                character: Math.max(0, label.endCol - 1),
              },
            },
          },
          message: label.message,
        }))
      : undefined,
    // The compiler's own fixes, carried through to `onCodeAction`. LSP
    // round-trips `data` untouched, so the quick fix is the compiler's
    // answer rather than this server's reading of the message.
    data: d.suggestions?.length
      ? { suggestions: d.suggestions, version: doc.version }
      : undefined,
  };
}

export function toDocumentSymbol(symbol: engine.EngineDocumentSymbol): DocumentSymbol {
  return {
    name: symbol.name,
    detail: symbol.detail === "" ? undefined : symbol.detail,
    // LSP 3.17 numbers `SymbolKind` 1–26 on both sides.
    kind: symbol.kind as SymbolKind,
    range: symbol.range,
    selectionRange: symbol.selectionRange,
    children: symbol.children.map(toDocumentSymbol),
  };
}

/** Places `symbol` among `siblings` in source order, inside the innermost
 * one whose range contains it (a `variant` in a `namespace`). */
export function insertSymbol(siblings: DocumentSymbol[], symbol: DocumentSymbol): void {
  const before = (a: engine.EnginePosition, b: engine.EnginePosition) =>
    a.line < b.line || (a.line === b.line && a.character <= b.character);
  const parent = siblings.find(
    (candidate) =>
      before(candidate.range.start, symbol.range.start) &&
      before(symbol.range.end, candidate.range.end),
  );
  if (parent) {
    insertSymbol((parent.children ??= []), symbol);
    return;
  }
  const at = siblings.findIndex((sibling) => !before(sibling.range.start, symbol.range.start));
  siblings.splice(at < 0 ? siblings.length : at, 0, symbol);
}

/**
 * The compiler's own fixes, off the diagnostic's `data`.
 *
 * A suggestion that names a replacement is a quick fix outright — the span
 * and the text both come from the compiler, so nothing here has to
 * recognise a message by its shape.
 */
export function suggestedFixes(
  doc: TextDocument,
  diag: Diagnostic,
  hasVersionedWorkspaceEditCapability: boolean,
): CodeAction[] {
  const data = diag.data as
    | { suggestions?: ttc.TtcSuggestion[]; version?: number }
    | undefined;
  const actions: CodeAction[] = [];
  if (data?.version !== doc.version) return actions;
  for (const suggestion of data?.suggestions ?? []) {
    const edit = suggestion.edit;
    if (!edit) continue;
    const range: Range = {
      start: { line: edit.line - 1, character: Math.max(0, edit.col - 1) },
      end: {
        line: edit.endLine - 1,
        character: Math.max(0, edit.endCol - 1),
      },
    };
    actions.push({
      // The compiler's own sentence names the fix. A title built from the
      // replacement text was readable while every fix was one identifier;
      // an inserted arm block is not a title (TASK-216).
      title: suggestion.message,
      kind: CodeActionKind.QuickFix,
      diagnostics: [diag],
      isPreferred: actions.length === 0,
      edit: hasVersionedWorkspaceEditCapability
        ? {
            documentChanges: [
              TextDocumentEdit.create({ uri: doc.uri, version: data.version }, [
                TextEdit.replace(range, edit.replacement),
              ]),
            ],
          }
        : {
            changes: {
              [doc.uri]: [TextEdit.replace(range, edit.replacement)],
            },
          },
    });
  }
  return actions;
}
