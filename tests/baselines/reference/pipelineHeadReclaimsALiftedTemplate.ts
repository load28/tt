//// [pipelineHeadReclaimsALiftedTemplate.tt] ////
const a = `v=${n}` |> f;


//// [pipelineHeadReclaimsALiftedTemplate.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
const a = $tt_ap(`v=${n}`, f);
