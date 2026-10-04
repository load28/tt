/** Wire answer shapes shared by engine consumers; no session state. */
import type { CompletionItemKind, CompletionItemTag } from "vscode-languageserver/node";

/** A position/range as both LSP and the engine protocol speak them. */
export interface EnginePosition {
  line: number;
  character: number;
}

export interface EngineRange {
  start: EnginePosition;
  end: EnginePosition;
}

export interface EngineLocation {
  path: string;
  range: EngineRange;
}

export interface EngineHover {
  signature: string;
  documentation: string;
  range: EngineRange;
}

export interface EngineCompletionItem {
  range?: EngineRange | null;
  labelDetails?: { detail?: string | null; description?: string | null } | null;
  label: string;
  source?: string | null;
  detail?: string | null;
  /** LSP 3.17 `CompletionItemKind`, as the service classified the entry;
   * null when it did not say. */
  kind?: CompletionItemKind | null;
  /** LSP 3.17 `CompletionItemTag`s: `1` (deprecated). */
  tags?: CompletionItemTag[];
  sortText: string;
  insertText?: string | null;
  filterText?: string | null;
  snippet?: boolean;
}

export interface EngineCompletionList {
  items: EngineCompletionItem[];
  member: boolean;
  /** Set when the list was answered from a completion probe; carried back
   * to `completionResolve` so the entry resolves against the same text. */
  probe: number | null;
}

export interface EngineCompletionDetail {
  signature: string;
  documentation: string;
  /** Edits accepting the entry also makes — an auto-import's import
   * declaration. Absent when there are none. */
  additionalEdits?: { range: EngineRange; newText: string }[];
}

export interface EngineRenameEdit extends EngineLocation {
  /** What the engine wants written, with RENAME_PLACEHOLDER standing in
   * for the new name; null for the bare name. */
  newText: string | null;
}

export interface EngineSignatureHelp {
  signatures: {
    label: string;
    documentation: string;
    parameters: { label: [number, number]; documentation: string }[];
  }[];
  activeSignature: number;
  activeParameter: number;
}

export interface EngineDiagnostic {
  range: EngineRange;
  message: string;
  code: number;
  severity: "error" | "warning" | "information" | "hint";
  /** TypeScript's unused and deprecated suggestions carry these; absent
   * when there are none. */
  tags?: ("unnecessary" | "deprecated")[];
  /** Secondary labeled spans ("the piped value is produced here"), absent
   * when the diagnostic has only its primary range. `path` names another
   * file; without it the span is in the diagnostic's own file. */
  related?: { range: EngineRange; message: string; path?: string }[];
}

export interface EngineTtSymbol {
  kind: "variant" | "case" | "field";
  range: EngineRange;
  name: string;
  variantName: string;
  /** The declaration in tt syntax — the hover's code block. */
  signature: string;
  /** One sentence about what it is and where it came from. */
  detail: string;
  definition: EngineLocation | null;
  binds: boolean;
}

/** One thing tt has to say about a range that is not an error. */
export interface EngineTtHint {
  /** What kind of hint it is — switch on this, not on the message. */
  kind: "unreachableArm";
  range: EngineRange;
  message: string;
}

export interface EngineTtCompletion {
  label: string;
  kind: "case" | "field" | "literal" | "wildcard";
  detail: string;
  /** True when an arm of this match already covers the case. */
  covered: boolean;
  /** The source range the item replaces when it is not the word at the
   * position: the string literal a literal pattern is written in. */
  range?: EngineRange | null;
}

/** What completion at a position is, read from the buffer's tokens. */
export interface EngineTtCompletions {
  /** The pattern completions tt owns there; empty elsewhere. */
  items: EngineTtCompletion[];
  /** Set when the cursor completes a member name: `receiver` is the path
   * of names before the `.` (`Result`, `ns.Shape`), or null for any other
   * expression (`f().`, `x |> .`). */
  member: { receiver: string | null } | null;
  /** The tt keywords whose construct can be written there, with
   * TypeScript's rank for a keyword. */
  keywords: EngineTtKeyword[];
  pattern: boolean;
}

/** A tt keyword the engine found valid at a position. */
export interface EngineTtKeyword {
  label: string;
  sortText: string;
}

export interface EngineSemanticToken {
  range: EngineRange;
  /** An LSP standard token-type string ("keyword", "enumMember", ...). */
  kind: string;
  /** LSP standard token-modifier strings ("declaration", ...); absent from
   * a compiler that predates them. */
  modifiers?: string[];
}

/** How a request ended: an engine result, an engine error (the session is
 * fine, the request failed), or null — the server itself is unavailable. */
export type EngineAnswer = { result: unknown } | { error: string } | null;

export interface EngineDocumentSymbol {
  name: string;
  detail: string;
  /** The LSP `SymbolKind` number. */
  kind: number;
  range: EngineRange;
  selectionRange: EngineRange;
  children: EngineDocumentSymbol[];
}

export interface EngineClassifiedToken {
  range: EngineRange;
  type: string;
  modifiers: string[];
}

/** A byte span in the buffer. */
export interface EngineSpan {
  start: number;
  end: number;
}

/** One variant visible in a buffer, from the compiler's own declaration
 * table (`declarations`, cli.md) — local, imported (aliases applied) and
 * built-in, under exactly the compiler's shadowing. */
export interface EngineVariantDecl {
  name: string;
  generics: string;
  origin: "local" | "imported" | "builtin";
  specifier: string | null;
  nameSpan: EngineSpan | null;
  span: EngineSpan | null;
  cases: EngineCaseDecl[];
}

/** One case of an [EngineVariantDecl]. */
export interface EngineCaseDecl {
  tag: string;
  nameSpan: EngineSpan | null;
  span: EngineSpan | null;
  unit: boolean;
  fields: { name: string; optional: boolean; ty: string }[];
}

/** One `match` site of the buffer: its keyword and the byte of the body's
 * closing `}` — the arm-insertion point. */
export interface EngineMatchSite {
  keyword: number;
  bodyOpen: number;
  bodyClose: number;
}

/** The compiler's declaration surface for one buffer. */
export interface EngineDeclarations {
  variants: EngineVariantDecl[];
  matches: EngineMatchSite[];
}
