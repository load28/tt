//// [aTryInAConciseArrowInsideAResultBlockTargetsTheArrow.tt] ////
declare function get(n: number): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
export const r = result {
const f = (n: number) => ({ kind: "Ok" as const, value: try get(n) + 1 });
const x = try get(1);
return f(x);
};


//// [aTryInAConciseArrowInsideAResultBlockTargetsTheArrow.ts]
declare function get(n: number): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: {
        kind: "Err";
        error: string;
    } | {
        kind: "Ok";
        value: number;
    };
});
$tt_v0: {
  const f = (n: number) => {
    let $tt_v1: number;
    const $tt_v2 = ("Ok" as const);
    const $tt_t0 = get(n);
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v1 = $tt_t0.value;
    return ({ kind: $tt_v2, value: $tt_v1 + 1 });
  };
const $tt_t1 = get(1);
if (!("value" in $tt_t1)) {
  $tt_v0 = $tt_t1;
  break $tt_v0;
}
const x = $tt_t1.value;
{
  const $tt_a0 = { value: { kind: "Ok" as const, value: f(x) } };
  $tt_v0 = $tt_a0.value;
  break $tt_v0;
}
}
export const r = $tt_v0;
