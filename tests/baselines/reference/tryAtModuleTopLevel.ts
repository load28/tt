//// [tryAtModuleTopLevel.tt] ////
// A `try` that no function encloses is rejected with one reason, whatever
// its form or the region it stands in: at a module's or a namespace's top
// level there is no function for its `return` to leave. A value-form `try`
// in a match arm, an array, or a conditional branch there used to get the
// generic "expression context" wording, and a statement `try` in a match
// block arm the value-region wording.
import type { TResult } from "@tt/std";
declare function read(n: number): TResult<number, string>;
variant V { A, B }
declare const v: V;
declare const flag: boolean;
export const x = match (v) { A => try read(1), B => 0 };
export const y = try read(2);
export const z = [try read(3)];
export const w = match (v) { A => { const q = try read(4); return q; }, B => 0 };
export const u = flag ? try read(5) : 0;
for (const i of [try read(6)]) {}
namespace N {
  export const n = match (v) { A => try read(7), B => 0 };
  const m = try read(8);
}

