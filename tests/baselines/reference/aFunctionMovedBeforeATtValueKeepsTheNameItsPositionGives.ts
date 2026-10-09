//// [aFunctionMovedBeforeATtValueKeepsTheNameItsPositionGives.tt] ////
variant O { A(n: number), B }
function g(cb: Function, n: number) { return cb.name + ":" + n; }
const obj = { m(cb: Function, n: number) { return cb.name + ":" + n; } };
function run(o: O, c: boolean) {
  const a = g(() => 1, match (o) { A(n) => n, B => 0 });
  const b = obj.m(function () {}, match (o) { A(n) => n, B => 0 });
  const d = { k: class {}, "a-b": () => 2, v: match (o) { A(n) => n, B => 0 } };
  const t = c ? (() => 1) : match (o) { A => 1, B => 2 };
  const l = (function () {}) || match (o) { A => 1, B => 2 };
  return [a, b, d.k.name, d["a-b"].name, (t as Function).name, l.name];
}
console.log(JSON.stringify(run(O.A(1), true)));


//// [aFunctionMovedBeforeATtValueKeepsTheNameItsPositionGives.ts]
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
function g(cb: Function, n: number) { return cb.name + ":" + n; }
const obj = { m(cb: Function, n: number) { return cb.name + ":" + n; } };
function run(o: O, c: boolean) {
  let $tt_v0: string;
  const $tt_v1: typeof g = (g);
  const $tt_v2: Function = ((void 0, () => 1));
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = $tt_v1($tt_v2, n);
        break;
      }
      case "B": {
        $tt_v0 = $tt_v1($tt_v2, 0);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const a = $tt_v0;
  let $tt_v3: string;
  const $tt_v5: Function = ((void 0, function () {}));
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v3 = obj.m($tt_v5, n);
        break;
      }
      case "B": {
        $tt_v3 = obj.m($tt_v5, 0);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const b = $tt_v3;
  let $tt_v6: number;
  const $tt_v7 = (({ "k": class {} })["k"]);
  {
    const $tt_m = o;
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
  const d = { k: $tt_v7, "a-b": () => 2, v: $tt_v6 };
  let $tt_v10: (() => number) | (number);
  if (c) {
    const $tt_a0 = { value: (void 0, () => 1) };
    $tt_v10 = $tt_a0.value;
  } else {
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v10 = 1;
          break;
        }
        case "B": {
          $tt_v10 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  }
  
  const t = $tt_v10;
  let $tt_v12: () => void;
  let $tt_v13: () => void;
  if ($tt_v13 = ({ value: (void 0, function () {}) }).value) {
    $tt_v12 = $tt_v13;
  } else {
    let $tt_v11: number;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v11 = 1;
          break;
        }
        case "B": {
          $tt_v11 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v12 = $tt_v13 || $tt_v11;
  }
  
  const l = $tt_v12;
  return [a, b, d.k.name, d["a-b"].name, (t as Function).name, l.name];
}
console.log(JSON.stringify(run(O.A(1), true)));
