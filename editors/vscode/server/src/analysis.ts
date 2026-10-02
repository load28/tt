/* --------------------------------------------------------------------------
 * Text-shape utilities for the language server.
 *
 * What lives here is the word at the cursor and the identifier rule. What
 * deliberately does NOT live here any more is tt *semantics* — which
 * variants are visible, what a match is over, what a case's fields are —
 * or the cursor's syntactic context. Those used to be a second, text-based
 * implementation of the compiler's rules and could disagree with it
 * (docs/design/rust-parity-analysis.md GAP-3, TASK-558); the compiler now
 * answers them itself through the server's `declarations` and
 * `ttCompletions` methods.
 * ----------------------------------------------------------------------- */

/** Reserved words — language.md §7. Not usable as variant names, tags, fields. */
export const RESERVED = new Set(
  (
    "async await break case catch class const continue debugger default " +
    "delete do else enum export extends false finally for function if " +
    "import in instanceof let new null of return static super switch this " +
    "throw true try typeof var variant void while with yield"
  ).split(" "),
);

const ID_START = /[A-Za-z_$]/;
const ID_CHAR = /[A-Za-z0-9_$]/;

/** ASCII identifier that is not a reserved word (language.md §1, §7). */
export function isIdent(word: string): boolean {
  return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(word) && !RESERVED.has(word);
}

export interface WordAt {
  word: string;
  start: number;
  end: number;
}

/** Identifier covering `offset` (offset may sit at its end). */
export function wordAt(src: string, offset: number): WordAt | null {
  let start = offset;
  while (start > 0 && ID_CHAR.test(src[start - 1])) start--;
  let end = offset;
  while (end < src.length && ID_CHAR.test(src[end])) end++;
  if (start === end) return null;
  const word = src.slice(start, end);
  if (!ID_START.test(word[0])) return null;
  return { word, start, end };
}
