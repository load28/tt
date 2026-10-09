//// [tsconfig.json] ////
{
  "compilerOptions": {
    "target": "es2022",
    "module": "esnext",
    "moduleResolution": "bundler",
    "jsx": "preserve",
    "strict": true,
    "noEmit": true
  }
}

//// [view.ttx] ////
declare global {
  namespace JSX {
    interface IntrinsicElements { [name: string]: unknown }
    type Element = unknown;
  }
}
export variant S { A, B }
declare const x: S;
declare const s: S;
declare const c: boolean;
declare function g(f: () => unknown): unknown;
export function a() { return <ul>{() => <li>{match (x) { A => 1, B => 2 }}</li>}{match (s) { A => 3, B => 4 }}</ul>; }
export function b() { return <ul>{() => (match (x) { A => 1, B => 2 })}{match (s) { A => 3, B => 4 }}</ul>; }
export function c1() { return <ul a={() => <li>{match (x) { A => 1, B => 2 }}</li>} b={match (s) { A => 3, B => 4 }}/>; }
export function d() { return <ul><b>{() => <li>{match (x) { A => 1, B => 2 }}</li>}</b>{match (s) { A => 3, B => 4 }}</ul>; }
export function e() { return <ul>{g(() => <li>{match (x) { A => 1, B => 2 }}</li>)}{c && match (s) { A => 3, B => 4 }}</ul>; }
export function h() { return <ul>{(() => <li>{match (x) { A => 1, B => 2 }}</li>)()}{match (s) { A => 3, B => 4 }}</ul>; }


//// [view.tsx]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}
declare global {
  namespace JSX {
    interface IntrinsicElements { [name: string]: unknown }
    type Element = unknown;
  }
}
export type S =
  | { kind: "A" }
  | { kind: "B" };
export const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
declare const x: S;
declare const s: S;
declare const c: boolean;
declare function g(f: () => unknown): unknown;
export function a() { let $tt_v0: number;
const $tt_v1 = ((void 0, () => {
  let $tt_v2: number;
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": $tt_v2 = 0; break;
      case "B": $tt_v2 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return <li>{($tt_v2 === 0 ? 1 : 2)}</li>;
}));
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "A": $tt_v0 = 0; break;
    case "B": $tt_v0 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
return <ul>{$tt_v1}{($tt_v0 === 0 ? 3 : 4)}</ul>; }
export function b() { let $tt_v3: number;
const $tt_v4 = ((void 0, () => {
  let $tt_v5: number;
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": $tt_v5 = 0; break;
      case "B": $tt_v5 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return (($tt_v5 === 0 ? 1 : 2));
}));
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "A": $tt_v3 = 0; break;
    case "B": $tt_v3 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
return <ul>{$tt_v4}{($tt_v3 === 0 ? 3 : 4)}</ul>; }
export function c1() { let $tt_v6: number;
const $tt_v7 = ((void 0, () => {
  let $tt_v8: number;
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": $tt_v8 = 0; break;
      case "B": $tt_v8 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return <li>{($tt_v8 === 0 ? 1 : 2)}</li>;
}));
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "A": $tt_v6 = 0; break;
    case "B": $tt_v6 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
return <ul a={$tt_v7} b={($tt_v6 === 0 ? 3 : 4)}/>; }
export function d() { let $tt_v9: number;
const $tt_v10 = (<b>{() => {
  let $tt_v11: number;
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": $tt_v11 = 0; break;
      case "B": $tt_v11 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return <li>{($tt_v11 === 0 ? 1 : 2)}</li>;
}}</b>);
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "A": $tt_v9 = 0; break;
    case "B": $tt_v9 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
return <ul>{$tt_v10}{($tt_v9 === 0 ? 3 : 4)}</ul>; }
export function e() { let $tt_v15: (number) | (false);
const $tt_v14 = (g(() => {
  let $tt_v16: number;
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": $tt_v16 = 0; break;
      case "B": $tt_v16 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return <li>{($tt_v16 === 0 ? 1 : 2)}</li>;
}));
let $tt_v13: boolean;
if ($tt_v13 = c) {
  let $tt_v12: number;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v12 = 3;
        break;
      }
      case "B": {
        $tt_v12 = 4;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v15 = $tt_v13 && $tt_v12;
} else {
  $tt_v15 = $tt_v13;
}

return <ul>{$tt_v14}{$tt_v15}</ul>; }
export function h() { let $tt_v17: number;
const $tt_v18 = ((() => {
  let $tt_v19: number;
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": $tt_v19 = 0; break;
      case "B": $tt_v19 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return <li>{($tt_v19 === 0 ? 1 : 2)}</li>;
})());
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "A": $tt_v17 = 0; break;
    case "B": $tt_v17 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
return <ul>{$tt_v18}{($tt_v17 === 0 ? 3 : 4)}</ul>; }
