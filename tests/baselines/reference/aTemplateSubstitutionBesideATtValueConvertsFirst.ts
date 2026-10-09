//// [aTemplateSubstitutionBesideATtValueConvertsFirst.tt] ////
variant V { A, B }
const arr = [1, 2];
function get(): V { arr.push(3); return V.A; }
export function f() {
  return `${arr}|${match (get()) { A => "a", B => "b" }}`;
}
console.log(f());


//// [aTemplateSubstitutionBesideATtValueConvertsFirst.ts]
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
const arr = [1, 2];
function get(): V { arr.push(3); return V.A; }
export function f() {
  let $tt_v0: number;
  const $tt_v1 = (`${arr}`);
  {
    const $tt_m = get();
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      case "B": $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return `${$tt_v1}|${($tt_v0 === 0 ? "a" : "b")}`;
}
console.log(f());
