//// [aSeparatorFollowsARestrictedProduction2.tt] ////
declare const o: { m(x: unknown): unknown };
declare const k: "m";
declare const c: boolean;
declare const u: unknown;
declare function f(): void;
declare const v: number;
export function* h() {
  yield
  v |> o.m
}


//// [aSeparatorFollowsARestrictedProduction2.ts]
declare const o: { m(x: unknown): unknown };
declare const k: "m";
declare const c: boolean;
declare const u: unknown;
declare function f(): void;
declare const v: number;
export function* h() {
  yield
  ;(($tt_v, $tt_r) => $tt_r.m($tt_v))(v, (o))
}
