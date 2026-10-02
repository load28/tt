//// [pipelineAwaitInHeadRunsInTheSurroundingAsyncContext.tt] ////

const upper = (s: string) => s.toUpperCase();
async function main() {
  const v = await Promise.resolve("ok") |> upper |> .concat("!");
  console.log(v);
}
await main();

export {};


//// [pipelineAwaitInHeadRunsInTheSurroundingAsyncContext.ts]

const upper = (s: string) => s.toUpperCase();
async function main() {
  const v = (($tt_v, $tt_f) => $tt_f($tt_v))(await Promise.resolve("ok"), upper).concat("!");
  console.log(v);
}
await main();

export {};
