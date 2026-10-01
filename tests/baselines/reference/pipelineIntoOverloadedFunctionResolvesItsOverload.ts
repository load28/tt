//// [main.tt] ////
export {};
function conv(x: string): number;
function conv(x: number): string;
function conv(x: string | number): number | string {
  return typeof x === "string" ? x.length : String(x);
}
function keep<T>(value: T): T {
  return value;
}
const log: string[] = [];
function noted<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function param(s: string): number {
  return s |> conv;
}
function local(): number {
  const q = "abc";
  return q |> conv;
}
function member(o: { s: string }): number {
  return o.s |> conv;
}
const arrow = (s: string): number => s |> conv;
const mapped: number[] = ["a", "bb"].map((s) => s |> conv);
const chained: string = "four" |> conv |> conv;
const wrapped: number = noted("head", "five") |> (conv);
const generic: number = noted("head", 5) |> keep<number> |> conv |> conv;
const composed = flow |> String |> conv;
const composedNumber: number = composed(123);
console.log(param("xy"), local(), member({ s: "z" }), arrow("abcd"), mapped, chained, wrapped, generic, composedNumber, log.join());

//// [twin.ts] ////
export {};
function conv(x: string): number;
function conv(x: number): string;
function conv(x: string | number): number | string {
  return typeof x === "string" ? x.length : String(x);
}
function keep<T>(value: T): T {
  return value;
}
const log: string[] = [];
function noted<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function param(s: string): number {
  return conv(s);
}
function local(): number {
  const q = "abc";
  return conv(q);
}
function member(o: { s: string }): number {
  return conv(o.s);
}
const arrow = (s: string): number => conv(s);
const mapped: number[] = ["a", "bb"].map((s) => conv(s));
const chained: string = conv(conv("four"));
const wrapped: number = conv(noted("head", "five"));
const generic: number = conv(conv(keep<number>(noted("head", 5))));
const composed = (v: unknown) => conv(String(v));
const composedNumber: number = composed(123);
console.log(param("xy"), local(), member({ s: "z" }), arrow("abcd"), mapped, chained, wrapped, generic, composedNumber, log.join());

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [main.ts]
import { $tt_fl } from "./tt/runtime.js";
export {};
function conv(x: string): number;
function conv(x: number): string;
function conv(x: string | number): number | string {
  return typeof x === "string" ? x.length : String(x);
}
function keep<T>(value: T): T {
  return value;
}
const log: string[] = [];
function noted<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function param(s: string): number {
  return (($tt_v, $tt_f) => $tt_f($tt_v))(s, conv);
}
function local(): number {
  const q = "abc";
  return (($tt_v, $tt_f) => $tt_f($tt_v))(q, conv);
}
function member(o: { s: string }): number {
  return (($tt_v, $tt_f) => $tt_f($tt_v))(o.s, conv);
}
const arrow = (s: string): number => (($tt_v, $tt_f) => $tt_f($tt_v))(s, conv);
const mapped: number[] = ["a", "bb"].map((s) => (($tt_v, $tt_f) => $tt_f($tt_v))(s, conv));
const chained: string = (($tt_v, $tt_f) => $tt_f($tt_v))(conv("four"), conv);
const wrapped: number = (($tt_v, $tt_f) => $tt_f($tt_v))(noted("head", "five"), (conv));
const generic: number = (($tt_v, $tt_f) => $tt_f($tt_v))((($tt_v, $tt_f) => $tt_f($tt_v))((($tt_v, $tt_f) => $tt_f($tt_v))(noted("head", 5), keep<number>), conv), conv);
const composed = $tt_fl(String, (($tt_f) => ($tt_v) => $tt_f($tt_v))(conv));
const composedNumber: number = composed(123);
console.log(param("xy"), local(), member({ s: "z" }), arrow("abcd"), mapped, chained, wrapped, generic, composedNumber, log.join());
\ No newline at end of file

//// [twin.ts]
export {};
function conv(x: string): number;
function conv(x: number): string;
function conv(x: string | number): number | string {
  return typeof x === "string" ? x.length : String(x);
}
function keep<T>(value: T): T {
  return value;
}
const log: string[] = [];
function noted<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function param(s: string): number {
  return conv(s);
}
function local(): number {
  const q = "abc";
  return conv(q);
}
function member(o: { s: string }): number {
  return conv(o.s);
}
const arrow = (s: string): number => conv(s);
const mapped: number[] = ["a", "bb"].map((s) => conv(s));
const chained: string = conv(conv("four"));
const wrapped: number = conv(noted("head", "five"));
const generic: number = conv(conv(keep<number>(noted("head", 5))));
const composed = (v: unknown) => conv(String(v));
const composedNumber: number = composed(123);
console.log(param("xy"), local(), member({ s: "z" }), arrow("abcd"), mapped, chained, wrapped, generic, composedNumber, log.join());
