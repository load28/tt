//// [anInitializerInsideACallbackStillLowersToStatements.tt] ////
declare function f(cb: () => number): void;
f(() => { const x = match (1) { 1 => 1, _ => 0 }; return x; });


//// [anInitializerInsideACallbackStillLowersToStatements.ts]
declare function f(cb: () => number): void;
f(() => { let $tt_v0: number;
{
  const $tt_m = 1;
  switch ($tt_m) {
    case 1: {
      $tt_v0 = 1;
      break;
    }
    default: {
      $tt_v0 = 0;
      break;
    }
  }
}
const x = $tt_v0; return x; });
