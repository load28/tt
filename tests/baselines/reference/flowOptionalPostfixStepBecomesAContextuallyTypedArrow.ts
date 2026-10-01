//// [flowOptionalPostfixStepBecomesAContextuallyTypedArrow.tt] ////
const f = flow |> parse |> ?.value?.toFixed(1);


//// [flowOptionalPostfixStepBecomesAContextuallyTypedArrow.ts]
var $tt_fl: <A extends unknown[], B, C>(
  f: (...a: A) => B,
  g: (b: B) => C,
) => (...a: A) => C = function (f, g) {
  return (...a) => g(f(...a));
};
const f = $tt_fl(parse, (($tt_v) => ($tt_v)?.value?.toFixed(1)));
