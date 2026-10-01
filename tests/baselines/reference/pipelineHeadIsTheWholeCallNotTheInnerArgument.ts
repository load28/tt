//// [pipelineHeadIsTheWholeCallNotTheInnerArgument.tt] ////
const y = f(a(b) |> g);


//// [pipelineHeadIsTheWholeCallNotTheInnerArgument.ts]
const y = f((($tt_v, $tt_f) => $tt_f($tt_v))(a(b), g));
