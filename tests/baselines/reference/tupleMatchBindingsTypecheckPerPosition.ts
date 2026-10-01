//// [tupleMatchBindingsTypecheckPerPosition.tt] ////

variant Left { A(n: number), B }
variant Right { C(s: string), D }
function f(l: Left, r: Right): string {
  return match (l, r) {
    (A(n), C(s)) => s.repeat(n),
    (A(n), D) => n.toFixed(0),
    (B, C(s)) => s,
    (B, D) => "",
  };
}

export {};


//// [tupleMatchBindingsTypecheckPerPosition.ts]
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

type Left =
  | { kind: "A"; n: number }
  | { kind: "B" };
const Left = {
  A: (n: number): Left => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
type Right =
  | { kind: "C"; s: string }
  | { kind: "D" };
const Right = {
  C: (s: string): Right => ({ kind: "C", s }),
  D: { kind: "D" } as const,
};
function f(l: Left, r: Right): string {
  let $tt_v0: string;
  {
    const $tt_m0 = l;
    const $tt_m1 = r;
    do {
      if ($tt_m0.kind === "A" && $tt_m1.kind === "C") {
        const { n } = $tt_m0;
        const { s } = $tt_m1;
        $tt_v0 = s.repeat(n);
        break;
      }
      if ($tt_m0.kind === "A" && $tt_m1.kind === "D") {
        const { n } = $tt_m0;
        $tt_v0 = n.toFixed(0);
        break;
      }
      if ($tt_m0.kind === "B" && $tt_m1.kind === "C") {
        const { s } = $tt_m1;
        $tt_v0 = s;
        break;
      }
      if ($tt_m0.kind === "B" && $tt_m1.kind === "D") {
        $tt_v0 = "";
        break;
      }
      throw new Error("tt match: unexpected case " + "[" + $tt_show($tt_m0) + "," + $tt_show($tt_m1) + "]");
    } while (false);
  }
  return $tt_v0;
}

export {};
