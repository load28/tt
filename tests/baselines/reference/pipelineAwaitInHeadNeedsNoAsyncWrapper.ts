//// [pipelineAwaitInHeadNeedsNoAsyncWrapper.tt] ////
async function f(p: Promise<string>) {
  return await p |> norm;
}


//// [pipelineAwaitInHeadNeedsNoAsyncWrapper.ts]
async function f(p: Promise<string>) {
  return (($tt_v, $tt_f) => $tt_f($tt_v))(await p, norm);
}
