//// [jumpsOwnedInsideAResultBlockStillCompile.tt] ////
import type { TResult } from "@tt/std";
declare const x: TResult<number, string>;
function f() { for (;;) { const r = result { const v = try x; inner: for (;;) { if (v) break inner; continue inner; } for (;;) { break; } return v; }; } }

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [jumpsOwnedInsideAResultBlockStillCompile.ts]
import type { TResult } from "./tt/index.js";
declare const x: TResult<number, string>;
function f() { for (;;) { let $tt_v0: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: number;
});
$tt_v0: {
  const $tt_t0 = x;
  if (!("value" in $tt_t0)) {
    $tt_v0 = $tt_t0;
    break $tt_v0;
  }
  const v = $tt_t0.value; inner: for (;;) { if (v) break inner; continue inner; } for (;;) { break; } { const $tt_a0 = { value: { kind: "Ok" as const, value: v } }; $tt_v0 = $tt_a0.value; break $tt_v0; }
}
const r = $tt_v0; } }
