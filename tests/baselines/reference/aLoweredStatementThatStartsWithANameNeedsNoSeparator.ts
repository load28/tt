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

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [aLoweredStatementThatStartsWithANameNeedsNoSeparator.ts]
import { $tt_ap } from "./tt/runtime.js";
declare const o: { m(x: unknown): unknown };
declare const k: "m";
declare const c: boolean;
declare const u: unknown;
declare function f(): void;
declare const v: number;
export function h() {
  const a = 1
  $tt_ap(v, String)
}
