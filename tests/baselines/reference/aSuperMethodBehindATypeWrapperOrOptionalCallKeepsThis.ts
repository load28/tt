//// [main.tt] ////
variant O { A(n: number), B }
const log: string[] = [];
class Base {
  m?(x: number) { return `m${x}${this.tag()}`; }
  tag() { return "#base"; }
}
function key(): "m" { log.push("key"); return "m"; }
class Derived extends Base {
  tag() { return "#derived"; }
  run(o: O) {
    const a = super.m!(match (o) { A(n) => n, B => 0 });
    const b = (super.m as (x: number) => string)(match (o) { A(n) => n * 2, B => 0 });
    const c = super.m?.(match (o) { A(n) => n * 3, B => 0 });
    const d = super[key()]!(match (o) { A(n) => (log.push("arm"), n * 4), B => 0 });
    return [a, b, c, d];
  }
}
class Overriding extends Base {
  m(x: number): string {
    return `over(${super.m?.(match (O.A(x) as O) { A(n) => n * 5, B => 0 })})`;
  }
}
console.log(new Derived().run(O.A(1)).join(" "), log.join(","), new Overriding().m(1));


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
const log: string[] = [];
class Base {
  m?(x: number) { return `m${x}${this.tag()}`; }
  tag() { return "#base"; }
}
function key(): "m" { log.push("key"); return "m"; }
class Derived extends Base {
  tag() { return "#derived"; }
  run(o: O) {
    let $tt_v0: number;
    {
      const $tt_m = o;
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
    const a = super.m!($tt_v0);
    let $tt_v1: number;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v1 = n * 2;
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
    const b = (super.m as (x: number) => string)($tt_v1);
    let $tt_v4: (string) | (undefined);
    const $tt_v3 = (super.m);
    if ($tt_v3 != null) {
      {
        const $tt_m = o;
        switch ($tt_m.kind) {
          case "A": {
            const { n } = $tt_m;
            $tt_v4 = $tt_v3.call(this, n * 3);
            break;
          }
          case "B": {
            $tt_v4 = $tt_v3.call(this, 0);
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
    } else {
      $tt_v4 = undefined;
    }
    
    const c = $tt_v4;
    let $tt_v5: string;
    const $tt_v7 = (key());
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v5 = super[$tt_v7]!((log.push("arm"), n * 4));
          break;
        }
        case "B": {
          $tt_v5 = super[$tt_v7]!(0);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    const d = $tt_v5;
    return [a, b, c, d];
  }
}
class Overriding extends Base {
  m(x: number): string {
    let $tt_v10: (string) | (undefined);
    const $tt_v9 = (super.m);
    if ($tt_v9 != null) {
      {
        const $tt_m = O.A(x) as O;
        switch ($tt_m.kind) {
          case "A": {
            const { n } = $tt_m;
            $tt_v10 = $tt_v9.call(this, n * 5);
            break;
          }
          case "B": {
            $tt_v10 = $tt_v9.call(this, 0);
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
    } else {
      $tt_v10 = undefined;
    }
    
    return `over(${$tt_v10})`;
  }
}
console.log(new Derived().run(O.A(1)).join(" "), log.join(","), new Overriding().m(1));
