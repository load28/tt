//// [pipelineGenericUserFunctionsInstantiate.tt] ////
const wrap = <T,>(v: T): T[] => [v];
const arr: number[][] = 3 |> wrap |> wrap;

export {};


//// [pipelineGenericUserFunctionsInstantiate.ts]
const wrap = <T,>(v: T): T[] => [v];
const arr: number[][] = (($tt_v, $tt_f) => $tt_f($tt_v))(wrap(3), wrap);

export {};
