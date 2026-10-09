//// [aResultBlockAsASpreadOperandIsLowered.tt] ////
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
function r(): R<number[]> { return { kind: "Ok", value: [1, 2] }; }
function f() { return { ...result { const q = try r(); return q; } }; }
console.log(JSON.stringify(f()));


//// [aResultBlockAsASpreadOperandIsLowered.ts]
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
function r(): R<number[]> { return { kind: "Ok", value: [1, 2] }; }
function f() { let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number[];
});
$tt_v0: {
  const $tt_t0 = r();
  if (!("value" in $tt_t0)) {
    $tt_v0 = $tt_t0;
    break $tt_v0;
  }
  const q = $tt_t0.value; { $tt_v0 = { kind: "Ok" as const, value: q }; break $tt_v0; }
}
return { ...$tt_v0 }; }
console.log(JSON.stringify(f()));
