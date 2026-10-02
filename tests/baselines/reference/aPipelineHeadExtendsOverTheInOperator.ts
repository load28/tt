//// [main.tt] ////
export {};
class Box {}
const o: Record<string, number> = { a: 1, b: 2 };
const box: unknown = new Box();
const keys = (value: object): string[] => Object.keys(value);
console.log("a" in o |> String, box instanceof Box |> String, 2 > 1 |> String);
for (const k in o |> keys) console.log(k);
for (let i = 0; "a" in o |> Boolean; i++) {
  console.log("loop", i);
  if (i > 0) break;
}

//// [twin.ts] ////
export {};
class Box {}
const o: Record<string, number> = { a: 1, b: 2 };
const box: unknown = new Box();
const keys = (value: object): string[] => Object.keys(value);
console.log(String("a" in o), String(box instanceof Box), String(2 > 1));
for (const k in keys(o)) console.log(k);
for (let i = 0; Boolean("a" in o); i++) {
  console.log("loop", i);
  if (i > 0) break;
}


//// [main.ts]
export {};
class Box {}
const o: Record<string, number> = { a: 1, b: 2 };
const box: unknown = new Box();
const keys = (value: object): string[] => Object.keys(value);
console.log(String("a" in o), (($tt_v, $tt_f) => $tt_f($tt_v))(box instanceof Box, String), (($tt_v, $tt_f) => $tt_f($tt_v))(2 > 1, String));
for (const k in (($tt_v, $tt_f) => $tt_f($tt_v))(o, keys)) console.log(k);
for (let i = 0; Boolean("a" in o); i++) {
  console.log("loop", i);
  if (i > 0) break;
}
\ No newline at end of file

//// [twin.ts]
export {};
class Box {}
const o: Record<string, number> = { a: 1, b: 2 };
const box: unknown = new Box();
const keys = (value: object): string[] => Object.keys(value);
console.log(String("a" in o), String(box instanceof Box), String(2 > 1));
for (const k in keys(o)) console.log(k);
for (let i = 0; Boolean("a" in o); i++) {
  console.log("loop", i);
  if (i > 0) break;
}
