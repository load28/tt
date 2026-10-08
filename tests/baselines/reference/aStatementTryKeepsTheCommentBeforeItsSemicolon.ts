//// [aStatementTryKeepsTheCommentBeforeItsSemicolon.tt] ////
import type { TResult } from "@tt/std";
declare function r(): TResult<number, string>;
export function f(): TResult<number, string> {
  try r() /* validated */;
  try r() // note
  ;
  return { kind: "Ok", value: 1 };
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aStatementTryKeepsTheCommentBeforeItsSemicolon.ts]
import type { TResult } from "./tt/index.js";
declare function r(): TResult<number, string>;
export function f(): TResult<number, string> {
  const $tt_t0 = r() /* validated */;
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const $tt_t1 = r() // note
  ;
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  return { kind: "Ok", value: 1 };
}
