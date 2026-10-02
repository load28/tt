//// [aPipelineAsTheUnbracedBodyOfAStatementHeaderStartsAfterTheHeader.tt] ////
declare const c: boolean, x: number;
declare function g(n: number): any;
declare const xs: AsyncIterable<number>;
if (c) x |> g;
if (c) x |> g; else x |> g;
while (c) x |> g;
for (;;) x |> g;
for (const a of [1]) a |> g;
async function h() { for await (const v of xs) v |> g; }
const y = (c) |> g;


//// [aPipelineAsTheUnbracedBodyOfAStatementHeaderStartsAfterTheHeader.ts]
declare const c: boolean, x: number;
declare function g(n: number): any;
declare const xs: AsyncIterable<number>;
if (c) (($tt_v, $tt_f) => $tt_f($tt_v))(x, g);
if (c) (($tt_v, $tt_f) => $tt_f($tt_v))(x, g); else (($tt_v, $tt_f) => $tt_f($tt_v))(x, g);
while (c) (($tt_v, $tt_f) => $tt_f($tt_v))(x, g);
for (;;) (($tt_v, $tt_f) => $tt_f($tt_v))(x, g);
for (const a of [1]) (($tt_v, $tt_f) => $tt_f($tt_v))(a, g);
async function h() { for await (const v of xs) (($tt_v, $tt_f) => $tt_f($tt_v))(v, g); }
const y = (($tt_v, $tt_f) => $tt_f($tt_v))((c), g);
