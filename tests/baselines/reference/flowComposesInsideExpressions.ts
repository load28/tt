//// [flowComposesInsideExpressions.tt] ////
const a = xs.map(flow |> parse |> double);
const b = `${flow |> f |> g}`;


//// [flowComposesInsideExpressions.ts]
var $tt_fl: <A extends unknown[], B, C>(
  f: (...a: A) => B,
  g: (b: B) => C,
) => (...a: A) => C = function (f, g) {
  return (...a) => g(f(...a));
};
const a = xs.map((($tt_g, $tt_f) => $tt_fl($tt_g, ($tt_v) => $tt_f($tt_v)))(parse, double));
const b = `${(($tt_g, $tt_f) => $tt_fl($tt_g, ($tt_v) => $tt_f($tt_v)))(f, g)}`;
