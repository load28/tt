//// [aSpreadAppliesToTheWholePipeline.tt] ////
const xs = [[1, 2], [3]];
const flat = (x: number[][]) => x.flat();
const g = (...a: unknown[]) => a.length;
const arr = [...xs |> flat];
const count = g(...xs |> flat);
const obj = { ...({ a: 1 }) |> ((o: { a: number }) => ({ ...o, b: 2 })) };
console.log(JSON.stringify(arr), count, JSON.stringify(obj));


//// [aSpreadAppliesToTheWholePipeline.ts]
const xs = [[1, 2], [3]];
const flat = (x: number[][]) => x.flat();
const g = (...a: unknown[]) => a.length;
const arr = [...(($tt_v, $tt_f) => $tt_f($tt_v))(xs, flat)];
const count = g(...(($tt_v, $tt_f) => $tt_f($tt_v))(xs, flat));
const obj = { ...((o: { a: number }) => ({ ...o, b: 2 }))(({ a: 1 })) };
console.log(JSON.stringify(arr), count, JSON.stringify(obj));
