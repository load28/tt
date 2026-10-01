//// [flowStepCanBeAParenthesizedArrow.tt] ////
const f = flow |> parse |> (n => n + 1);


//// [flowStepCanBeAParenthesizedArrow.ts]
var $tt_fl: <A extends unknown[], B, C>(
  f: (...a: A) => B,
  g: (b: B) => C,
) => (...a: A) => C = function (f, g) {
  return (...a) => g(f(...a));
};
const f = $tt_fl(parse, (n => n + 1));
