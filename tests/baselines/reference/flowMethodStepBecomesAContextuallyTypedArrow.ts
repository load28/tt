//// [flowMethodStepBecomesAContextuallyTypedArrow.tt] ////
const f = flow |> parse |> .toFixed(1);


//// [flowMethodStepBecomesAContextuallyTypedArrow.ts]
var $tt_fl: <A extends unknown[], B, C>(
  f: (...a: A) => B,
  g: (b: B) => C,
) => (...a: A) => C = function (f, g) {
  return (...a) => g(f(...a));
};
const f = $tt_fl(parse, (($tt_v) => ($tt_v).toFixed(1)));
