//// [parenthesizedTernaryHeadCompiles.tt] ////
const a = (c ? x : y) |> f;


//// [parenthesizedTernaryHeadCompiles.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
const a = $tt_ap((c ? x : y), f);
