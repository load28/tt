//// [flowFirstStepOptionalChain.tt] ////
// A `flow` composition's first step is the composed function itself, so an
// optional-chain step, whose function may be absent, cannot be first. A
// parenthesized function that makes the optional call can.
declare const scale: { by(n: number): number } | undefined;
const direct = flow |> scale?.by |> String;
const nested = flow |> scale?.["by"];
const wrapped = flow |> ((n: number) => scale?.by(n)) |> String;

