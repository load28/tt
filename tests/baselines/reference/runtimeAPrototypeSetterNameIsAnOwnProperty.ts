//// [runtimeAPrototypeSetterNameIsAnOwnProperty.tt] ////

variant V { __proto__(x: number), Other }
variant U { __proto__, Other }
variant W { A(__proto__: number, other?: number), B }
const v = V.__proto__(1);
console.log(JSON.stringify(v), Object.keys(V).join(","), Object.getPrototypeOf(V) === Object.prototype);
console.log(JSON.stringify(U.__proto__), Object.getPrototypeOf(U) === Object.prototype);
const w = W.A(5);
console.log(Object.keys(w).join(","), Object.getPrototypeOf(w) === Object.prototype);
console.log(match (v) { __proto__(x) => x, Other => 0 }, match (w) { A(__proto__) => __proto__, B => 0 });

export {};


//// [runtimeAPrototypeSetterNameIsAnOwnProperty.ts]
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
  | { kind: "__proto__"; x: number }
  | { kind: "Other" };
const V = {
  ["__proto__"]: (x: number): V => ({ kind: "__proto__", x }),
  Other: { kind: "Other" } as const,
};
type U =
  | { kind: "__proto__" }
  | { kind: "Other" };
const U = {
  ["__proto__"]: { kind: "__proto__" } as const,
  Other: { kind: "Other" } as const,
};
type W =
  | { kind: "A"; __proto__: number; other?: number }
  | { kind: "B" };
const W = {
  A: (__proto__: number, other?: number): W => ({ kind: "A", __proto__, ...(other === undefined ? {} : { other }) }),
  B: { kind: "B" } as const,
};
const v = V.__proto__(1);
console.log(JSON.stringify(v), Object.keys(V).join(","), Object.getPrototypeOf(V) === Object.prototype);
console.log(JSON.stringify(U.__proto__), Object.getPrototypeOf(U) === Object.prototype);
const w = W.A(5);
console.log(Object.keys(w).join(","), Object.getPrototypeOf(w) === Object.prototype);
let $tt_v0: number;
{
  const $tt_m = v;
  switch ($tt_m.kind) {
    case "__proto__": {
      const { x } = $tt_m;
      $tt_v0 = x;
      break;
    }
    case "Other": {
      $tt_v0 = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
{
  const $tt_m = w;
  switch ($tt_m.kind) {
    case "A": {
      const { __proto__ } = $tt_m;
      console.log($tt_v0, __proto__);
      break;
    }
    case "B": {
      console.log($tt_v0, 0);
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}


export {};
