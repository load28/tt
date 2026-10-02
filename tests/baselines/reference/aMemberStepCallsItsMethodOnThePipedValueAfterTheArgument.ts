//// [aMemberStepCallsItsMethodOnThePipedValueAfterTheArgument.tt] ////
declare const x: any;
declare function g(): any;
declare const n: number;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
export const v = g() |> .m(match (n) { 0 => 1, _ => 2 });


//// [aMemberStepCallsItsMethodOnThePipedValueAfterTheArgument.ts]
declare const x: any;
declare function g(): any;
declare const n: number;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
let $tt_v0;
do {
  const $tt_v4 = g();
  let $tt_v1: number;
  const $tt_v3 = ($tt_v4);
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 0: {
        $tt_v1 = 1;
        break;
      }
      default: {
        $tt_v1 = 2;
        break;
      }
    }
  }
  $tt_v0 = $tt_v3.m($tt_v1);
  break;
} while (false);
export const v = $tt_v0;
