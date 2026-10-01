//// [pipelineHeadReclaimsALiftedTemplate.tt] ////
const a = `v=${n}` |> f;


//// [pipelineHeadReclaimsALiftedTemplate.ts]
const a = (($tt_v, $tt_f) => $tt_f($tt_v))(`v=${n}`, f);
