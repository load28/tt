//// [aDeclarationTryBindsToThePrimaryExpressionLikeTheValueForm.tt] ////
declare function total(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function g() { const x = try total() * 1.1; return x; }


//// [aDeclarationTryBindsToThePrimaryExpressionLikeTheValueForm.ts]
declare function total(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function g() { let $tt_v0: number;
const $tt_t0 = total();
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
$tt_v0 = $tt_t0.value;
const x = $tt_v0 * 1.1; return x; }
