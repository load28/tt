//// [aConditionalOperandOwnsItsBranchInAPipelineHead.tt] ////
declare function f(a: any, b?: any): any;
declare function g(): any;
declare class C { constructor(a: any); }
declare const n: number;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
export const v = g() && f(match (n) { 0 => 1, _ => 2 }) |> String;


//// [aConditionalOperandOwnsItsBranchInAPipelineHead.ts]
declare function f(a: any, b?: any): any;
declare function g(): any;
declare class C { constructor(a: any); }
declare const n: number;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
let $tt_v0: string;
do {
  let $tt_v5;
  let $tt_v4;
  let $tt_v3;
  if ($tt_v3 = g()) {
    let $tt_v1: number;
    const $tt_v2: typeof f = (f);
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
    $tt_v4 = $tt_v3 && $tt_v2($tt_v1);
  } else {
    $tt_v4 = $tt_v3;
  }
  $tt_v5 = $tt_v4;
  $tt_v0 = String($tt_v5);
  break;
} while (false);
export const v = $tt_v0;
