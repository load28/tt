//// [aSeparatorIsWrittenAtEveryNestingDepth.tt] ////
declare const o: { m(x: unknown): unknown };
declare const k: "m";
declare const c: boolean;
declare const u: unknown;
declare function f(): void;
declare const v: number;
export class A {
  m(x: number) { return x }
  run() {
    const w = 1
    w |> this.m
  }
}
export const g = () => {
  const w = 2
  w |> o.m
}
export const t = `${(() => { const w = 3
  w |> o.m
  return w })()}`


//// [aSeparatorIsWrittenAtEveryNestingDepth.ts]
declare const o: { m(x: unknown): unknown };
declare const k: "m";
declare const c: boolean;
declare const u: unknown;
declare function f(): void;
declare const v: number;
export class A {
  m(x: number) { return x }
  run() {
    const w = 1
    ;(($tt_v, $tt_r) => $tt_r.m($tt_v))(w, (this))
  }
}
export const g = () => {
  const w = 2
  ;(($tt_v, $tt_r) => $tt_r.m($tt_v))(w, (o))
}
export const t = `${(() => { const w = 3
  ;(($tt_v, $tt_r) => $tt_r.m($tt_v))(w, (o))
  return w })()}`
