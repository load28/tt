//// [aSeparatorFollowsARestrictedProduction1.tt] ////
declare const o: { m(x: unknown): unknown };
declare const k: "m";
declare const c: boolean;
declare const u: unknown;
declare function f(): void;
declare const v: number;
export function* h() {
  return
  v |> o.m
}


//// [aSeparatorFollowsARestrictedProduction1.ts]
declare const o: { m(x: unknown): unknown };
declare const k: "m";
declare const c: boolean;
declare const u: unknown;
declare function f(): void;
declare const v: number;
export function* h() {
  return
  ;(($tt_v, $tt_r) => $tt_r.m($tt_v))(v, (o))
}
