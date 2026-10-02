//// [anUnexpectedValueGuardReportsEveryScrutineeType.tt] ////

variant V { A, B }
const cyclic: { kind: string; self?: unknown } = { kind: "Z" };
cyclic.self = cyclic;
const values: unknown[] = [
  2n, -5n, Symbol("s"), "zz", 3, NaN, undefined, null, true,
  { toJSON() { throw new Error("toJSON"); } }, cyclic, () => 1, { kind: "Q" },
];
function statement(n: unknown): string {
  return match (n) { 1n => "one", "a" => "a" };
}
function chain(n: unknown, ok: boolean): string {
  return match (n) { 1n if ok => "one", "a" => "a" };
}
type Item = { run: (x: number) => number };
const pair2 = (a: Item, b: Item) => a.run(1) + b.run(1);
function inline(n: unknown): number {
  return pair2(
    match (n) { 1n => ({ run: x => x }), "a" => ({ run: x => x + 1 }) },
    match (n) { 1n => ({ run: x => x }), "a" => ({ run: x => x + 1 }) },
  );
}
function kind(v: V): string {
  return match (v) { A => "a", B => "b" };
}
function pair(a: V, b: V): string {
  return match (a, b) { (A, A) => "aa", (B, _) => "b", (A, B) => "ab" };
}
const report = (run: () => unknown) => {
  try { run(); console.log("returned"); }
  catch (error) { console.log(error instanceof Error ? error.message : "not an Error: " + String(error)); }
};
for (const value of values) {
  report(() => statement(value));
  report(() => chain(value, true));
  report(() => inline(value));
  if (value !== null && value !== undefined) report(() => kind(value as V));
}
report(() => pair(V.A, 7n as unknown as V));
report(() => pair(V.A, { kind: "Nope" } as unknown as V));

export {};


//// [anUnexpectedValueGuardReportsEveryScrutineeType.ts]
function $tt_raise(error: unknown): never { throw error; }
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

type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const cyclic: { kind: string; self?: unknown } = { kind: "Z" };
cyclic.self = cyclic;
const values: unknown[] = [
  2n, -5n, Symbol("s"), "zz", 3, NaN, undefined, null, true,
  { toJSON() { throw new Error("toJSON"); } }, cyclic, () => 1, { kind: "Q" },
];
function statement(n: unknown): string {
  let $tt_v0: string;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1n: {
        $tt_v0 = "one";
        break;
      }
      case "a": {
        $tt_v0 = "a";
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}
function chain(n: unknown, ok: boolean): string {
  let $tt_v1: string;
  {
    const $tt_m = n;
    do {
      if ($tt_m === 1n) {
        if (ok) {
          $tt_v1 = "one";
          break;
        }
      }
      if ($tt_m === "a") {
        $tt_v1 = "a";
        break;
      }
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    } while (false);
  }
  return $tt_v1;
}
type Item = { run: (x: number) => number };
const pair2 = (a: Item, b: Item) => a.run(1) + b.run(1);
function inline(n: unknown): number {
  let $tt_subject_2;
  let $tt_subject_3;
  
  return pair2(
    ($tt_subject_2 = n, ($tt_subject_2 === 1n) ? ({ run: x => x }) : ($tt_subject_2 === "a") ? ({ run: x => x + 1 }) : $tt_raise(new Error("tt match: unexpected literal " + $tt_show($tt_subject_2)))),
    ($tt_subject_3 = n, ($tt_subject_3 === 1n) ? ({ run: x => x }) : ($tt_subject_3 === "a") ? ({ run: x => x + 1 }) : $tt_raise(new Error("tt match: unexpected literal " + $tt_show($tt_subject_3)))),
  );
}
function kind(v: V): string {
  let $tt_v5: string;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v5 = "a";
        break;
      }
      case "B": {
        $tt_v5 = "b";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v5;
}
function pair(a: V, b: V): string {
  let $tt_v6: string;
  {
    const $tt_m0 = a;
    const $tt_m1 = b;
    do {
      if ($tt_m0.kind === "A" && $tt_m1.kind === "A") {
        $tt_v6 = "aa";
        break;
      }
      if ($tt_m0.kind === "B") {
        $tt_v6 = "b";
        break;
      }
      if ($tt_m0.kind === "A" && $tt_m1.kind === "B") {
        $tt_v6 = "ab";
        break;
      }
      throw new Error("tt match: unexpected case " + "[" + $tt_show($tt_m0) + "," + $tt_show($tt_m1) + "]");
    } while (false);
  }
  return $tt_v6;
}
const report = (run: () => unknown) => {
  try { run(); console.log("returned"); }
  catch (error) { console.log(error instanceof Error ? error.message : "not an Error: " + String(error)); }
};
for (const value of values) {
  report(() => statement(value));
  report(() => chain(value, true));
  report(() => inline(value));
  if (value !== null && value !== undefined) report(() => kind(value as V));
}
report(() => pair(V.A, 7n as unknown as V));
report(() => pair(V.A, { kind: "Nope" } as unknown as V));

export {};
