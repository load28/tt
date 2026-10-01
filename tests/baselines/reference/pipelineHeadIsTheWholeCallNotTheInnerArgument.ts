//// [pipelineHeadIsTheWholeCallNotTheInnerArgument.tt] ////
const y = f(a(b) |> g);


//// [pipelineHeadIsTheWholeCallNotTheInnerArgument.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
const y = f($tt_ap(a(b), g));
