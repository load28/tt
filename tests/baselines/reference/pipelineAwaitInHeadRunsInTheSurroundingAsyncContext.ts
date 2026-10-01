//// [pipelineAwaitInHeadRunsInTheSurroundingAsyncContext.tt] ////

const upper = (s: string) => s.toUpperCase();
async function main() {
  const v = await Promise.resolve("ok") |> upper |> .concat("!");
  console.log(v);
}
await main();

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [pipelineAwaitInHeadRunsInTheSurroundingAsyncContext.ts]
import { $tt_ap } from "./tt/runtime.js";

const upper = (s: string) => s.toUpperCase();
async function main() {
  const v = $tt_ap(await Promise.resolve("ok"), upper).concat("!");
  console.log(v);
}
await main();

export {};
