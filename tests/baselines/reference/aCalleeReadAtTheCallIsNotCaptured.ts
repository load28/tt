//// [aCalleeReadAtTheCallIsNotCaptured.tt] ////
variant V { A(n: number), B }
function get(): V { return V.A(3); }
export function direct() {
  const secret = "local";
  return eval(match (get()) { A => "typeof secret", B => "0" });
}
class Base {
  constructor(public n: number) {}
  m(x: number) { return x * 10; }
}
class Derived extends Base {
  constructor(v: V) { super(match (v) { A(n) => n, B => 0 }); }
  m(x: number) { return x; }
  piped(v: V) { return match (v) { A(n) => n, B => 0 } |> super.m; }
  named(v: V) { return super.m(match (v) { A(n) => n, B => 0 }); }
  computed(v: V) { return super["m"](match (v) { A(n) => n, B => 0 }); }
}
const d = new Derived(V.A(4));
console.log(direct());
console.log(d.n, d.piped(V.A(2)), d.named(V.A(3)), d.computed(V.B));


//// [aCalleeReadAtTheCallIsNotCaptured.ts]
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
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
function get(): V { return V.A(3); }
export function direct() {
  const secret = "local";
  let $tt_v0: number;
  {
    const $tt_m = get();
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      case "B": $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return eval(($tt_v0 === 0 ? "typeof secret" : "0"));
}
class Base {
  constructor(public n: number) {}
  m(x: number) { return x * 10; }
}
class Derived extends Base {
  constructor(v: V) { let $tt_v1: number;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v1 = n;
        break;
      }
      case "B": {
        $tt_v1 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  super($tt_v1); }
  m(x: number) { return x; }
  piped(v: V) { let $tt_v2: number;
  do {
    let $tt_v6: number;
    {
      const $tt_m = v;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v6 = n;
          break;
        }
        case "B": {
          $tt_v6 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v2 = super.m($tt_v6);
    break;
  } while (false);
  return $tt_v2; }
  named(v: V) { let $tt_v3: number;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v3 = n;
        break;
      }
      case "B": {
        $tt_v3 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return super.m($tt_v3); }
  computed(v: V) { let $tt_v4: number;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v4 = n;
        break;
      }
      case "B": {
        $tt_v4 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return super["m"]($tt_v4); }
}
const d = new Derived(V.A(4));
console.log(direct());
console.log(d.n, d.piped(V.A(2)), d.named(V.A(3)), d.computed(V.B));
