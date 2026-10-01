//// [flowIsAContextualKeywordOnlyAtAPipelineHead.tt] ////
const a = (flow) |> f;
const b = o.flow |> f;
const c = flow() |> f;


//// [flowIsAContextualKeywordOnlyAtAPipelineHead.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
const a = $tt_ap((flow), f);
const b = $tt_ap(o.flow, f);
const c = $tt_ap(flow(), f);
