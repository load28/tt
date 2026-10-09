//// [anIfLetBoundToAnUnparenthesizedTry.tt] ////
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
