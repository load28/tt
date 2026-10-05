/* --------------------------------------------------------------------------
 * The list the server publishes for a buffer, from its diagnostic layers.
 *
 * `validate` in server.ts collects the layers for one validation generation
 * and publishes what `publishedDiagnostics` makes of them. An LSP publish
 * replaces the file's complete list (LSP 3.17,
 * `textDocument/publishDiagnostics`), so this is the whole rule for what
 * the editor shows.
 * ----------------------------------------------------------------------- */
import { Diagnostic, DiagnosticSeverity } from "vscode-languageserver/node";

/** One validation generation's layers, already on the buffer's text. */
export interface DiagnosticLayers {
  /** What tt decides from the text alone (`ttc --check`). */
  text: Diagnostic[];
  /** TypeScript's language-service diagnostics (`source: "ts"`). */
  service: Diagnostic[];
  /** The codes of compiler diagnostics the service states in TypeScript's
   * words (`engine.tsDiagnosticsAnswer`). */
  restates: string[];
  /** Original causes whose source was replaced by an editor repair. */
  retains?: { code: string; start: { line: number; character: number } }[];
  /** tt's hints, which are never problems. */
  hints: Diagnostic[];
  /** The typed compiler pass, when it answered. `replacesTypes` says it
   * checked the buffer's TypeScript, so its checker diagnostics stand in
   * for the service's. */
  typed: { diagnostics: Diagnostic[]; replacesTypes: boolean } | null;
}

/** An error or a warning — what the Problems panel counts — as opposed to
 * a suggestion the editor only fades or strikes through. */
export function isProblem(d: Diagnostic): boolean {
  return (
    d.severity === undefined ||
    d.severity === DiagnosticSeverity.Error ||
    d.severity === DiagnosticSeverity.Warning
  );
}

/** A diagnostic's rule, with the typed pass's `ts` prefix removed so that
 * `ts2322` and the service's `2322` name the same TypeScript rule. */
function ruleOf(d: Diagnostic): string {
  return String(d.code ?? "").replace(/^ts(?=\d+$)/, "");
}

/** Two layers state the same diagnostic when they report the same rule at
 * the same start. Anything else is a different diagnostic, even at the
 * same place: TypeScript reports several rules at one call. */
export function sameDiagnostic(left: Diagnostic, right: Diagnostic): boolean {
  const rule = ruleOf(left);
  return (
    rule !== "" &&
    rule === ruleOf(right) &&
    left.range.start.line === right.range.start.line &&
    left.range.start.character === right.range.start.character
  );
}

/**
 * Adds the typed pass's diagnostics to the other layers'.
 *
 * The passes overlap: variant exhaustiveness is decided from the text by
 * `--check` and from the type by the typed pass, and a checker diagnostic
 * is reported by the language service and by the typed pass. The typed
 * pass's statement replaces the other layer's statement of the same
 * diagnostic (`sameDiagnostic`); the typed pass's own diagnostics never
 * replace each other. When `replacesTypes`, the service's problems are
 * removed first, so a consequence the compiler suppresses cannot stay
 * visible; the service's suggestions (unused, deprecated) stay, since the
 * compiler reports none.
 */
function mergeTyped(into: Diagnostic[], typed: Diagnostic[], replacesTypes: boolean): Diagnostic[] {
  const base = replacesTypes
    ? into.filter((d) => !(d.source === "ts" && isProblem(d)))
    : into.slice();
  const merged = base.slice();
  const added: Diagnostic[] = [];
  for (const d of typed) {
    const stated = base.findIndex((other) => isProblem(other) && sameDiagnostic(other, d));
    if (stated >= 0) merged[stated] = d;
    else added.push(d);
  }
  return merged.concat(added);
}

/** The compiler's source-order contract: by start, then end, then code. */
function inSourceOrder(left: Diagnostic, right: Diagnostic): number {
  const start =
    left.range.start.line - right.range.start.line ||
    left.range.start.character - right.range.start.character;
  if (start !== 0) return start;
  const end =
    left.range.end.line - right.range.end.line ||
    left.range.end.character - right.range.end.character;
  if (end !== 0) return end;
  return String(left.code ?? "").localeCompare(String(right.code ?? ""));
}

/**
 * The list to publish.
 *
 * A syntax error in the TypeScript the user wrote is TypeScript's to
 * report, in its own words, as it is in a `.ts` file. The compiler's
 * layers state the same fact as the reason the file has no output; once
 * TypeScript has stated it, they would only state it twice, so a compiler
 * diagnostic whose code the service restates is left out. The typed pass is
 * merged last, and the result is put in source order so the Problems panel
 * agrees with the CLI whichever layer authored a rule.
 */
export function publishedDiagnostics(layers: DiagnosticLayers): Diagnostic[] {
  const restated = new Set(layers.restates);
  const stated = (d: Diagnostic) => d.source === "ts" || !restated.has(String(d.code ?? "")) ||
    layers.retains?.some((cause) => cause.code === String(d.code ?? "") &&
      cause.start.line === d.range.start.line && cause.start.character === d.range.start.character);
  let diagnostics = [...layers.text.filter(stated), ...layers.service, ...layers.hints];
  if (layers.typed !== null) {
    diagnostics = mergeTyped(
      diagnostics,
      layers.typed.diagnostics.filter(stated),
      layers.typed.replacesTypes,
    );
  }
  return diagnostics.sort(inSourceOrder);
}
