//// [pipelineMethodStepChainsPostfix.tt] ////
const t = s |> .trim() |> .split(",") |> f;


//// [pipelineMethodStepChainsPostfix.ts]
const t = (($tt_v, $tt_f) => $tt_f($tt_v))(s.trim().split(","), f);
