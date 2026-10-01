//// [runtimeGuardedAllWildcardArmIsDecidedByItsGuard.tt] ////

variant T { A, B }
type Item = { run: (x: number) => number };
function pair(a: Item, b: Item): number { return a.run(0) * 10 + b.run(0); }
function stmt(a: T, b: T, cond: boolean): number {
  return match (a, b) {
    (A, _) => 1,
    (_, _) if cond => 2,
    _ => 3,
  };
}
function last(a: T, b: T, cond: boolean): number {
  return match (a, b) {
    (A, _) => 1,
    (_, _) if cond => 2,
    (_, _) => 3,
  };
}
function selected(a: T, b: T, cond: boolean): number {
  let seen = 0;
  const consume = (item: Item) => { seen = item.run(0); };
  consume(match (a, b) {
    (A, _) => ({ run: x => x + 1 }),
    (_, _) if cond => ({ run: x => x + 2 }),
    (_, _) => ({ run: x => x + 3 }),
  });
  return seen;
}
function inline(a: T, b: T, cond: boolean): number {
  return pair(
    match (a, b) { (A, _) => ({ run: x => x + 1 }), (_, _) if cond => ({ run: x => x + 2 }), (_, _) => ({ run: x => x + 3 }) },
    match (b, a) { (A, _) => ({ run: x => x + 4 }), (_, _) if !cond => ({ run: x => x + 5 }), _ => ({ run: x => x + 6 }) },
  );
}
function literal(n: number, cond: boolean): string {
  return match (n) {
    1 if cond => "one",
    1 | 2 => "small",
    _ => "other",
  };
}
console.log(stmt(T.A, T.B, false), stmt(T.B, T.A, true), stmt(T.B, T.A, false));
console.log(last(T.A, T.B, false), last(T.B, T.A, true), last(T.B, T.A, false));
console.log(selected(T.A, T.B, false), selected(T.B, T.A, true), selected(T.B, T.A, false));
console.log(inline(T.A, T.B, true), inline(T.B, T.B, true), inline(T.B, T.A, false));
console.log(literal(1, true), literal(1, false), literal(3, true));

export {};


//// [runtimeGuardedAllWildcardArmIsDecidedByItsGuard.ts]

type T =
  | { kind: "A" }
  | { kind: "B" };
const T = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
type Item = { run: (x: number) => number };
function pair(a: Item, b: Item): number { return a.run(0) * 10 + b.run(0); }
function stmt(a: T, b: T, cond: boolean): number {
  let $tt_v0: number;
  {
    const $tt_m0 = a;
    const $tt_m1 = b;
    do {
      if ($tt_m0.kind === "A") {
        $tt_v0 = 1;
        break;
      }
      if (cond) {
        $tt_v0 = 2;
        break;
      }
      $tt_v0 = 3;
      break;
    } while (false);
  }
  return $tt_v0;
}
function last(a: T, b: T, cond: boolean): number {
  let $tt_v1: number;
  {
    const $tt_m0 = a;
    const $tt_m1 = b;
    do {
      if ($tt_m0.kind === "A") {
        $tt_v1 = 1;
        break;
      }
      if (cond) {
        $tt_v1 = 2;
        break;
      }
      $tt_v1 = 3;
      break;
    } while (false);
  }
  return $tt_v1;
}
function selected(a: T, b: T, cond: boolean): number {
  let seen = 0;
  const consume = (item: Item) => { seen = item.run(0); };
  const $tt_v3 = (consume);
  const $tt_m0 = a;
  const $tt_m1 = b;
  
  $tt_v3((($tt_m0.kind === "A") ? ({ run: x => x + 1 }) : (cond) ? ({ run: x => x + 2 }) : ({ run: x => x + 3 })));
  return seen;
}
function inline(a: T, b: T, cond: boolean): number {
  let $tt_subject_6;
  let $tt_subject_8;
  
  return pair(
    ($tt_subject_6 = a, b, ($tt_subject_6.kind === "A") ? ({ run: x => x + 1 }) : (cond) ? ({ run: x => x + 2 }) : ({ run: x => x + 3 })),
    ($tt_subject_8 = b, a, ($tt_subject_8.kind === "A") ? ({ run: x => x + 4 }) : (!cond) ? ({ run: x => x + 5 }) : ({ run: x => x + 6 })),
  );
}
function literal(n: number, cond: boolean): string {
  let $tt_v7: string;
  {
    const $tt_m = n;
    do {
      if ($tt_m === 1) {
        if (cond) {
          $tt_v7 = "one";
          break;
        }
      }
      if ($tt_m === 1 || $tt_m === 2) {
        $tt_v7 = "small";
        break;
      }
      $tt_v7 = "other";
      break;
    } while (false);
  }
  return $tt_v7;
}
console.log(stmt(T.A, T.B, false), stmt(T.B, T.A, true), stmt(T.B, T.A, false));
console.log(last(T.A, T.B, false), last(T.B, T.A, true), last(T.B, T.A, false));
console.log(selected(T.A, T.B, false), selected(T.B, T.A, true), selected(T.B, T.A, false));
console.log(inline(T.A, T.B, true), inline(T.B, T.B, true), inline(T.B, T.A, false));
console.log(literal(1, true), literal(1, false), literal(3, true));

export {};
