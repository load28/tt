//// [pipelineMethodStepChainsPostfix.tt] ////
const t = s |> .trim() |> .split(",") |> f;


//// [pipelineMethodStepChainsPostfix.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
const t = $tt_ap(s.trim().split(","), f);
