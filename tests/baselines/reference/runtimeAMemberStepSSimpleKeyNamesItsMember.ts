//// [runtimeAMemberStepSSimpleKeyNamesItsMember.tt] ////

const trace: string[] = [];
const obj = { m(x: number) { return x * 2; }, n(x: number) { return x * 5; } };
function h(n: number) { trace.push("head"); return n + 1; }
function getFns(): [(n: number) => number, string] { return [(n) => n * 3, "s"]; }
let key: "m" | "n" = "m";
function receiver() { trace.push("receiver"); key = "n"; return obj; }
const c = (flow |> obj["m"])(3);
const d = h(3) |> (() => obj)()["m"];
const e = h(3) |> getFns()[0];
const f = h(3) |> obj[`n`];
const g = h(3) |> receiver()[key];
console.log(c, d, e, f, g, JSON.stringify(trace));

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [runtimeAMemberStepSSimpleKeyNamesItsMember.ts]
import { $tt_ap } from "./tt/runtime.js";

const trace: string[] = [];
const obj = { m(x: number) { return x * 2; }, n(x: number) { return x * 5; } };
function h(n: number) { trace.push("head"); return n + 1; }
function getFns(): [(n: number) => number, string] { return [(n) => n * 3, "s"]; }
let key: "m" | "n" = "m";
function receiver() { trace.push("receiver"); key = "n"; return obj; }
const c = ((($tt_r) => ($tt_r["m"]).bind($tt_r))((obj)))(3);
const d = (($tt_v, $tt_r) => $tt_r["m"]($tt_v))(h(3), ((() => obj)()));
const e = (($tt_v, $tt_r) => $tt_r[0]($tt_v))(h(3), (getFns()));
const f = $tt_ap(h(3), obj[`n`]);
const g = (($tt_v, $tt_r) => $tt_r[key]($tt_v))(h(3), (receiver()));
console.log(c, d, e, f, g, JSON.stringify(trace));

export {};
