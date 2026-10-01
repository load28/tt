//// [aScriptLetElseKeepsItsBindingsGlobal.tt] ////
declare const o: { kind: "Some"; value: number } | { kind: "None" };
const Some(value) = o else { throw new Error(); };
var Some(value: other) = o else { throw new Error(); };
let None() = o else { throw new Error(); };


//// [aScriptLetElseKeepsItsBindingsGlobal.ts]
declare const o: { kind: "Some"; value: number } | { kind: "None" };
const $tt_t0$value = o;
if ($tt_t0$value.kind !== "Some") {
  throw new Error();
}
const { value } = $tt_t0$value;
{
  const $tt_t1 = o;
  if ($tt_t1.kind !== "Some") {
    throw new Error();
  }
  var { value: other } = $tt_t1;
}
{
  const $tt_t2 = o;
  if ($tt_t2.kind !== "None") {
    throw new Error();
  }
}
