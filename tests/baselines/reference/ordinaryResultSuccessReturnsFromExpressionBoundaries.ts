//// [ordinaryResultSuccessReturnsFromExpressionBoundaries.tt] ////
class Box { field = result { const value = try read(); return value; }; }
class SwitchBox { field = result { const value = try read(); switch (value) { case 0: return 0; default: return value; } }; }
function withDefault(value = result { const item = try read(); return item; }) { return value; }
function* values() { yield result { const item = try read(); return item; }; }
const text = `value=${result { const item = try read(); return item; }}`;


//// [ordinaryResultSuccessReturnsFromExpressionBoundaries.ts]
var $tt_expr: <T>(run: () => T) => T = function (run) { return run(); };
class Box { field = $tt_expr(() => {
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const value = $tt_t0.value; { return { kind: "Ok" as const, value: value }; }
  }); }
class SwitchBox { field = $tt_expr(() => {
  const $tt_t1 = read();
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  const value = $tt_t1.value; switch (value) { case 0: { return { kind: "Ok" as const, value: 0 }; } default: { return { kind: "Ok" as const, value: value }; } }
  }); }
function withDefault(value = $tt_expr(() => {
  const $tt_t2 = read();
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  const item = $tt_t2.value; { return { kind: "Ok" as const, value: item }; }
  })) { return value; }
function* values() { let $tt_v3;
$tt_v3: {
  const $tt_t3 = read();
  if (!("value" in $tt_t3)) {
    $tt_v3 = $tt_t3;
    break $tt_v3;
  }
  const item = $tt_t3.value; { $tt_v3 = { kind: "Ok" as const, value: item }; break $tt_v3; }
}
yield $tt_v3; }
let $tt_v4$text;
$tt_v4$text: {
  const $tt_t4 = read();
  if (!("value" in $tt_t4)) {
    $tt_v4$text = $tt_t4;
    break $tt_v4$text;
  }
  const item = $tt_t4.value; { $tt_v4$text = { kind: "Ok" as const, value: item }; break $tt_v4$text; }
}
const text = `value=${$tt_v4$text}`;
