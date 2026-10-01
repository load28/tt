//// [theRuntimeImportNeverDisplacesADirectiveOrAShebang3.tt] ////
declare const b: string;
"a" + b;
declare function f(n: number): number;
export const a = f(4) |> f |> .toFixed(1);


//// [theRuntimeImportNeverDisplacesADirectiveOrAShebang3.ts]
declare const b: string;
"a" + b;
declare function f(n: number): number;
export const a = (($tt_v, $tt_f) => $tt_f($tt_v))(f(4), f).toFixed(1);
