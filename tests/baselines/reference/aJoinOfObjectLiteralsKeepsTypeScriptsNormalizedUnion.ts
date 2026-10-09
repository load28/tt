//// [main.tt] ////
variant O { A(n: number), B }
function f(o: O) {
  const x = match (o) { A(n) => ({ a: n }), B => ({ b: "x" }) };
  return [x.a, x.b];
}
function g(o: O) {
  const y = match (o) { A(n) => ({ inner: { p: n } }), B => ({ inner: { q: "y" } }) };
  return [y.inner.p, y.inner.q];
}
console.log(JSON.stringify([f(O.A(1)), f(O.B), g(O.A(2)), g(O.B)]));


//// [main.ts]
var $tt_show: (value: unknown) => string = function (value) {
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
};
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
function f(o: O) {
  let $tt_v0: {
    a: number;
    b?: undefined;
} | {
    a?: undefined;
    b: string;
};
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = ({ a: n });
        break;
      }
      case "B": {
        $tt_v0 = ({ b: "x" });
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const x = $tt_v0;
  return [x.a, x.b];
}
function g(o: O) {
  let $tt_v1: {
    inner: {
        p: number;
        q?: undefined;
    };
} | {
    inner: {
        p?: undefined;
        q: string;
    };
};
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v1 = ({ inner: { p: n } });
        break;
      }
      case "B": {
        $tt_v1 = ({ inner: { q: "y" } });
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const y = $tt_v1;
  return [y.inner.p, y.inner.q];
}
console.log(JSON.stringify([f(O.A(1)), f(O.B), g(O.A(2)), g(O.B)]));
