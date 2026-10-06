//// [anIfLetBoundToAnUnparenthesizedTry.tt] ////
// The bound expression of an `if let` runs to the first `{` after a
// complete expression, and a value `try` is an expression: its operand
// needs no parentheses. A `try` followed by `{` is the statement form and
// is not a bound expression.
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
variant G { E(n: number), F }
const ok = (value: G): R<G> => ({ kind: "Ok", value });
const err = (error: string): R<G> => ({ kind: "Err", error });

function first(r: R<G>): R<number> {
    if let E(n) = try r {
        return { kind: "Ok", value: n };
    }
    return { kind: "Ok", value: 0 };
}

console.log(JSON.stringify(first(ok(G.E(4)))), JSON.stringify(first(ok(G.F))), JSON.stringify(first(err("no"))));
export {};


//// [anIfLetBoundToAnUnparenthesizedTry.ts]
// The bound expression of an `if let` runs to the first `{` after a
// complete expression, and a value `try` is an expression: its operand
// needs no parentheses. A `try` followed by `{` is the statement form and
// is not a bound expression.
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
type G =
  | { kind: "E"; n: number }
  | { kind: "F" };
const G = {
  E: (n: number): G => ({ kind: "E", n }),
  F: { kind: "F" } as const,
};
const ok = (value: G): R<G> => ({ kind: "Ok", value });
const err = (error: string): R<G> => ({ kind: "Err", error });

function first(r: R<G>): R<number> {
    {
      let $tt_t0; const $tt_t1 = r;
      if (!("value" in $tt_t1)) {
        return $tt_t1;
      }
      $tt_t0 = $tt_t1.value;
      if ($tt_t0.kind === "E") {
        const { n } = $tt_t0;
        return { kind: "Ok", value: n };
      }
    }
    return { kind: "Ok", value: 0 };
}

console.log(JSON.stringify(first(ok(G.E(4)))), JSON.stringify(first(ok(G.F))), JSON.stringify(first(err("no"))));
export {};
