//// [runtimeValuesHoistedOutOfUnbracedBodiesStayUnderTheirParent.tt] ////

variant S { A(n: number), B }
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const log: string[] = [];
function note(x: unknown) { log.push(String(x)); }
function f(s: S, c: boolean) {
  if (c) return match (s) { A(n) => n, B => 0 };
  return -1;
}
function loops(s: S, xs: number[]) {
  for (const q of xs) note(match (s) { A(n) => n + q, B => q });
  let i = 0;
  while (i++ < 2) note(match (s) { A(n) => n, B => -1 });
  outer: for (const q of match (s) { A(n) => [n, n + 1], B => [] }) { if (q > 5) continue outer; note(q); }
  do note(match (s) { A(n) => -n, B => 0 }); while (false);
  lbl: note(match (s) { A(n) => n * 10, B => 0 });
}
function tries(c: boolean, r: R): R {
  if (c) note(try r); else note("else");
  if (c) note(result { const v = try r; return v + 1; }.kind);
  if (c) for (let k = try r; k < 6; k++) note(k);
  return { kind: "Ok", value: 0 };
}
console.log(f(S.A(3), true), f(S.A(3), false), f(S.B, true));
loops(S.A(5), [1, 2]);
console.log(log.join(","));
log.length = 0;
console.log(tries(true, { kind: "Err", error: "e" }).kind, tries(false, { kind: "Ok", value: 1 }).kind, tries(true, { kind: "Ok", value: 4 }).kind);
console.log(log.join(","));

export {};


//// [runtimeValuesHoistedOutOfUnbracedBodiesStayUnderTheirParent.ts]
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

type S =
  | { kind: "A"; n: number }
  | { kind: "B" };
const S = {
  A: (n: number): S => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const log: string[] = [];
function note(x: unknown) { log.push(String(x)); }
function f(s: S, c: boolean) {
  if (c) {
    let $tt_v0: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v0 = n;
          break;
        }
        case "B": {
          $tt_v0 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    return $tt_v0;
  }
  return -1;
}
function loops(s: S, xs: number[]) {
  for (const q of xs) {
    const $tt_v2: typeof note = (note);
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v2(n + q);
          break;
        }
        case "B": {
          $tt_v2(q);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    
  }
  let i = 0;
  while (i++ < 2) {
    const $tt_v4: typeof note = (note);
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v4(n);
          break;
        }
        case "B": {
          $tt_v4(-1);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    
  }
  let $tt_v5: number[];
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        const $tt_a0 = { value: [n, n + 1] };
        $tt_v5 = $tt_a0.value;
        break;
      }
      case "B": {
        const $tt_a1 = { value: [] };
        $tt_v5 = $tt_a1.value;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  outer: for (const q of $tt_v5) { if (q > 5) continue outer; note(q); }
  do {
    const $tt_v7: typeof note = (note);
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v7(-n);
          break;
        }
        case "B": {
          $tt_v7(0);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    
  } while (false);
  lbl: {
    const $tt_v9: typeof note = (note);
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v9(n * 10);
          break;
        }
        case "B": {
          $tt_v9(0);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    
  }
}
function tries(c: boolean, r: R): R {
  if (c) {
    let $tt_v10: number;
    const $tt_v11: typeof note = (note);
    const $tt_t0 = r;
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v10 = $tt_t0.value;
    $tt_v11($tt_v10);
  } else note("else");
  if (c) {
    let $tt_v12: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
    const $tt_v13: typeof note = (note);
    $tt_v12: {
      const $tt_t1 = r;
      if (!("value" in $tt_t1)) {
        $tt_v12 = $tt_t1;
        break $tt_v12;
      }
      const v = $tt_t1.value; { $tt_v12 = { kind: "Ok" as const, value: v + 1 }; break $tt_v12; }
    }
    $tt_v13($tt_v12.kind);
  }
  if (c) {
    const $tt_t2 = r;
    if (!("value" in $tt_t2)) {
      return $tt_t2;
    }
    for (let k = $tt_t2.value; k < 6; k++) note(k);
  }
  return { kind: "Ok", value: 0 };
}
console.log(f(S.A(3), true), f(S.A(3), false), f(S.B, true));
loops(S.A(5), [1, 2]);
console.log(log.join(","));
log.length = 0;
console.log(tries(true, { kind: "Err", error: "e" }).kind, tries(false, { kind: "Ok", value: 1 }).kind, tries(true, { kind: "Ok", value: 4 }).kind);
console.log(log.join(","));

export {};
