//// [aLoweredStatementThatStartsWithANameNeedsNoSeparator.tt] ////
declare const o: { m(x: unknown): unknown };
declare const k: "m";
declare const c: boolean;
declare const u: unknown;
declare function f(): void;
declare const v: number;
export function h() {
  const a = 1
  v |> String
}


//// [aLoweredStatementThatStartsWithANameNeedsNoSeparator.ts]
declare const o: { m(x: unknown): unknown };
declare const k: "m";
declare const c: boolean;
declare const u: unknown;
declare function f(): void;
declare const v: number;
export function h() {
  const a = 1
  ;(($tt_v, $tt_f) => $tt_f($tt_v))(v, String)
}
