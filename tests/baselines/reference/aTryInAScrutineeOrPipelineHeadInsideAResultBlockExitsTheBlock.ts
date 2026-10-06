//// [aTryInAScrutineeOrPipelineHeadInsideAResultBlockExitsTheBlock.tt] ////
// A match's scrutinee and a pipeline's head run before the construct's own
// value regions (its arms and steps), as they do in a function body, so a
// `try` there exits the enclosing `result` block. A `try` inside a function
// written in an arm, a step, or an interpolation targets that function.
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
variant G { E, F }
const ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const err = <T,>(error: string): R<T> => ({ kind: "Err", error });
const double = (x: number) => x * 2;

function scrutinee(g: R<G>, c: boolean): R<number> {
    return result {
        const a = match (try g) { E => 1, F => 2 };
        const b = c ? match (try g) { E => 10, F => 20 } : 0;
        const d = match (try g, try g) { (E, E) => 100, _ => 200 };
        return a + b + d;
    };
}

function head(n: R<number>): R<number> {
    return result {
        const b = (try n) |> double;
        return b;
    };
}

function nested(n: R<number>, m: R<R<number>>): R<string> {
    return result {
        const v = try n;
        const e = m |> (x => try x);
        const f = match (n) { _ => (() => try n)() };
        const h = `${(() => try n)()}`;
        return JSON.stringify([v, e, f, h]);
    };
}

const show = (r: unknown) => JSON.stringify(r);
console.log(show(scrutinee(ok(G.E), true)), show(scrutinee(ok(G.F), false)), show(scrutinee(err("s"), true)));
console.log(show(head(ok(4))), show(head(err("h"))));
console.log(show(nested(ok(1), ok(ok(2)))));
export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [aTryInAScrutineeOrPipelineHeadInsideAResultBlockExitsTheBlock.ts]
import { $tt_ap } from "./tt/runtime.js";
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
// A match's scrutinee and a pipeline's head run before the construct's own
// value regions (its arms and steps), as they do in a function body, so a
// `try` there exits the enclosing `result` block. A `try` inside a function
// written in an arm, a step, or an interpolation targets that function.
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
type G =
  | { kind: "E" }
  | { kind: "F" };
const G = {
  E: { kind: "E" } as const,
  F: { kind: "F" } as const,
};
const ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const err = <T,>(error: string): R<T> => ({ kind: "Err", error });
const double = (x: number) => x * 2;

function scrutinee(g: R<G>, c: boolean): R<number> {
    let $tt_v0: R<number>;
    $tt_v0: {
      let $tt_v1: number;
      {
        let $tt_m; const $tt_t0 = g;
        if (!("value" in $tt_t0)) {
          $tt_v0 = $tt_t0;
          break $tt_v0;
        }
        $tt_m = $tt_t0.value;
        switch ($tt_m.kind) {
          case "E": {
            $tt_v1 = 1;
            break;
          }
          case "F": {
            $tt_v1 = 2;
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      const a = $tt_v1;
        let $tt_v4: number;
        if (c) {
          {
            let $tt_m; const $tt_t1 = g;
            if (!("value" in $tt_t1)) {
              $tt_v0 = $tt_t1;
              break $tt_v0;
            }
            $tt_m = $tt_t1.value;
            switch ($tt_m.kind) {
              case "E": {
                $tt_v4 = 10;
                break;
              }
              case "F": {
                $tt_v4 = 20;
                break;
              }
              default: {
                throw new Error("tt match: unexpected case " + $tt_show($tt_m));
              }
            }
          }
        } else {
          $tt_v4 = 0;
        }
        
        const b = $tt_v4;
        let $tt_v5: number;
        {
          let $tt_m0; const $tt_t2 = g;
          if (!("value" in $tt_t2)) {
            $tt_v0 = $tt_t2;
            break $tt_v0;
          }
          $tt_m0 = $tt_t2.value;
          let $tt_m1; const $tt_t3 = g;
          if (!("value" in $tt_t3)) {
            $tt_v0 = $tt_t3;
            break $tt_v0;
          }
          $tt_m1 = $tt_t3.value;
          do {
            if ($tt_m0.kind === "E" && $tt_m1.kind === "E") {
              $tt_v5 = 100;
              break;
            }
            $tt_v5 = 200;
            break;
          } while (false);
        }
        const d = $tt_v5;
        {
          $tt_v0 = { kind: "Ok" as const, value: a + b + d };
          break $tt_v0;
        }
    }
    return $tt_v0;
}

function head(n: R<number>): R<number> {
    let $tt_v6: R<number>;
    $tt_v6: {
      let $tt_v7: number;
      do {
        let $tt_v19: number;
        let $tt_v17: number;
        const $tt_t4 = n;
        if (!("value" in $tt_t4)) {
          $tt_v6 = $tt_t4;
          break $tt_v6;
        }
        $tt_v17 = $tt_t4.value;
        $tt_v19 = ($tt_v17);
        $tt_v7 = double($tt_v19);
        break;
      } while (false);
      const b = $tt_v7;
        {
          $tt_v6 = { kind: "Ok" as const, value: b };
          break $tt_v6;
        }
    }
    return $tt_v6;
}

function nested(n: R<number>, m: R<R<number>>): R<string> {
    let $tt_v8: R<string>;
    $tt_v8: {
      const $tt_t5 = n;
      if (!("value" in $tt_t5)) {
        $tt_v8 = $tt_t5;
        break $tt_v8;
      }
      const v = $tt_t5.value;
        const e = $tt_ap(m, ((x => {
          let $tt_v9: R<number>;
          const $tt_t6 = x;
          if (!("value" in $tt_t6)) {
            return $tt_t6;
          }
          $tt_v9 = $tt_t6.value;
          return $tt_v9;
        })));
        let $tt_v10: number | {
    kind: "Err";
    error: string;
};
        {
          const $tt_m = n;
          switch ($tt_m) {
            default: {
              $tt_v10 = ((() => {
                let $tt_v11: number;
                const $tt_t7 = n;
                if (!("value" in $tt_t7)) {
                  return $tt_t7;
                }
                $tt_v11 = $tt_t7.value;
                return $tt_v11;
              })());
              break;
            }
          }
        }
        const f = $tt_v10;
        const h = `${(() => {
          let $tt_v12: number;
          const $tt_t8 = n;
          if (!("value" in $tt_t8)) {
            return $tt_t8;
          }
          $tt_v12 = $tt_t8.value;
          return $tt_v12;
        })()}`;
        {
          $tt_v8 = { kind: "Ok" as const, value: JSON.stringify([v, e, f, h]) };
          break $tt_v8;
        }
    }
    return $tt_v8;
}

const show = (r: unknown) => JSON.stringify(r);
console.log(show(scrutinee(ok(G.E), true)), show(scrutinee(ok(G.F), false)), show(scrutinee(err("s"), true)));
console.log(show(head(ok(4))), show(head(err("h"))));
console.log(show(nested(ok(1), ok(ok(2)))));
export {};
