//// [pipelineAwaitInHeadNeedsNoAsyncWrapper.tt] ////
async function f(p: Promise<string>) {
  return await p |> norm;
}


//// [pipelineAwaitInHeadNeedsNoAsyncWrapper.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
async function f(p: Promise<string>) {
  return $tt_ap(await p, norm);
}
