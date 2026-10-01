//// [flowIsAContextualKeywordOnlyAtAPipelineHead.tt] ////
const a = (flow) |> f;
const b = o.flow |> f;
const c = flow() |> f;


//// [flowIsAContextualKeywordOnlyAtAPipelineHead.ts]
const a = (($tt_v, $tt_f) => $tt_f($tt_v))((flow), f);
const b = (($tt_v, $tt_f) => $tt_f($tt_v))(o.flow, f);
const c = (($tt_v, $tt_f) => $tt_f($tt_v))(flow(), f);
