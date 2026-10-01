//// [generatedBindingsNeverCaptureUserIdentifiers.tt] ////

variant O { S(v: number), N }
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const Ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const $tt_m = "m";
const $tt_m_1 = "m1";
const $tt_t0 = "t0";
const $tt_ap = "ap";
const $tt_fl = "fl";
const $tt_v = "v";
const $tt_r = "r";
const $tt_k = "k";
const obj = { tag: "o", add(n: number) { return n + this.tag + $tt_r + $tt_k + $tt_v; } };
const key = "add" as const;
const r = match (O.S(1)) { S(v) => v + $tt_m + $tt_m_1, N => "" };
function f(): R<string> { const a = try Ok(2); return Ok(a + $tt_t0); }
const xs = [1].map(x => x |> String);
const ys = [1].map(x => x |> obj.add);
const zs = [1].map(x => x |> obj[key]);
const g = flow |> ((n: number) => n + 1) |> .toFixed(1) |> Number |> obj.add;
console.log(r, JSON.stringify(f()), xs[0], ys[0], zs[0], g(1), $tt_ap, $tt_fl);

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [generatedBindingsNeverCaptureUserIdentifiers.ts]
import { $tt_ap as $tt_ap_1, $tt_fl as $tt_fl_1 } from "./tt/runtime.js";
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

type O =
  | { kind: "S"; v: number }
  | { kind: "N" };
const O = {
  S: (v: number): O => ({ kind: "S", v }),
  N: { kind: "N" } as const,
};
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const Ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const $tt_m = "m";
const $tt_m_1 = "m1";
const $tt_t0 = "t0";
const $tt_ap = "ap";
const $tt_fl = "fl";
const $tt_v = "v";
const $tt_r = "r";
const $tt_k = "k";
const obj = { tag: "o", add(n: number) { return n + this.tag + $tt_r + $tt_k + $tt_v; } };
const key = "add" as const;
let $tt_v0: string;
{
  const $tt_m_2 = O.S(1);
  switch ($tt_m_2.kind) {
    case "S": {
      const { v } = $tt_m_2;
      $tt_v0 = v + $tt_m + $tt_m_1;
      break;
    }
    case "N": {
      $tt_v0 = "";
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m_2));
    }
  }
}
const r = $tt_v0;
function f(): R<string> { const $tt_t0_1 = Ok(2);
if (!("value" in $tt_t0_1)) {
  return $tt_t0_1;
}
const a = $tt_t0_1.value; return Ok(a + $tt_t0); }
const xs = [1].map(x => $tt_ap_1(x, String));
const ys = [1].map(x => (($tt_v_1, $tt_r_1) => $tt_r_1.add($tt_v_1))(x, (obj)));
const zs = [1].map(x => (($tt_v_1, $tt_r_1) => $tt_r_1[key]($tt_v_1))(x, (obj)));
const g = $tt_fl_1($tt_fl_1($tt_fl_1(((n: number) => n + 1), (($tt_v_1) => ($tt_v_1).toFixed(1))), Number), (($tt_r_1) => ($tt_r_1.add).bind($tt_r_1))((obj)));
console.log(r, JSON.stringify(f()), xs[0], ys[0], zs[0], g(1), $tt_ap, $tt_fl);

export {};
