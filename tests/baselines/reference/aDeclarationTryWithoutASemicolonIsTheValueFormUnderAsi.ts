//// [aDeclarationTryWithoutASemicolonIsTheValueFormUnderAsi.tt] ////
declare function g(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function f() {
  const n = try g()
  return n;
}


//// [aDeclarationTryWithoutASemicolonIsTheValueFormUnderAsi.ts]
declare function g(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function f() {
  let $tt_v0: number;
  const $tt_t0 = g();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  const n = $tt_v0
  return n;
}
