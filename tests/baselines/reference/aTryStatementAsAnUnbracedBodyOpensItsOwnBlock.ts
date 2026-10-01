//// [aTryStatementAsAnUnbracedBodyOpensItsOwnBlock.tt] ////
declare function a(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function g() { if (true) try a(); return 1; }
function i() { while (true) try a(); }


//// [aTryStatementAsAnUnbracedBodyOpensItsOwnBlock.ts]
declare function a(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function g() { if (true) {
  const $tt_t0 = a();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
} return 1; }
function i() { while (true) {
  const $tt_t1 = a();
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
} }
