//// [aCommentBeforeTheEqualsOfATryDeclarationKeepsItsInitializer.tt] ////
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const ok = (n: number): R => ({ kind: "Ok", value: n });
function f(): R {
  const v //C
    = try ok(3);
  return ok(v + 1);
}
function g(): R {
  const w /* block */ = try ok(5);
  return ok(w * 2);
}
console.log(JSON.stringify(f()), JSON.stringify(g()));


//// [aCommentBeforeTheEqualsOfATryDeclarationKeepsItsInitializer.ts]
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const ok = (n: number): R => ({ kind: "Ok", value: n });
function f(): R {
  const $tt_t0 = ok(3);
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const v //C
    = $tt_t0.value;
  return ok(v + 1);
}
function g(): R {
  const $tt_t1 = ok(5);
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  const w /* block */ = $tt_t1.value;
  return ok(w * 2);
}
console.log(JSON.stringify(f()), JSON.stringify(g()));
