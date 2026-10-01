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
const a = xs.map($tt_fl(parse, (($tt_f) => ($tt_v) => $tt_f($tt_v))(double)));
const b = `${$tt_fl(f, (($tt_f) => ($tt_v) => $tt_f($tt_v))(g))}`;
