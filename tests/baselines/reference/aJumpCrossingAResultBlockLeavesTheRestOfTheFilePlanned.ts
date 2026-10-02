//// [aJumpCrossingAResultBlockLeavesTheRestOfTheFilePlanned.tt] ////
import type { TResult } from "@tt/std";
declare const x: TResult<number, string>;
function f() { for (;;) { const r = result { const v = try x; if (v) break; return v; }; } }
class C { y = try x; }

