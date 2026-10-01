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
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
declare const c: boolean, x: number;
declare function g(n: number): any;
declare const xs: AsyncIterable<number>;
if (c) $tt_ap(x, g);
if (c) $tt_ap(x, g); else $tt_ap(x, g);
while (c) $tt_ap(x, g);
for (;;) $tt_ap(x, g);
for (const a of [1]) $tt_ap(a, g);
async function h() { for await (const v of xs) $tt_ap(v, g); }
const y = $tt_ap((c), g);
