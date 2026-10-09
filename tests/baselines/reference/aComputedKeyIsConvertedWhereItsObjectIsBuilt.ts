//// [aComputedKeyIsConvertedWhereItsObjectIsBuilt.tt] ////
export {};
variant O { A, B }
const log: string[] = [];
const scr = () => { log.push("scrutinee"); return O.A as O; };
const key = { toString() { log.push("key->string"); return "kk"; } };
const obj = { [key as any]: match (scr()) { A => 1, B => 2 } };
console.log(log.join(", "), JSON.stringify(obj));
log.length = 0;
const part = { toString() { log.push("part->string"); return "x"; } };
const s = `${part}${match (scr()) { A => 1, B => 2 }}`;
console.log(log.join(", "), s);
log.length = 0;
const inline = `${{ toString() { log.push("inline->string"); return "y"; } }}${match (scr()) { A => 1, B => 2 }}`;
console.log(log.join(", "), inline);
log.length = 0;
const plain = `${{ a: 1 }}${[1, 2]}${match (scr()) { A => 1, B => 2 }}`;
console.log(log.join(", "), plain);


//// [aComputedKeyIsConvertedWhereItsObjectIsBuilt.ts]
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
export {};
type O =
  | { kind: "A" }
  | { kind: "B" };
const O = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const log: string[] = [];
const scr = () => { log.push("scrutinee"); return O.A as O; };
const key = { toString() { log.push("key->string"); return "kk"; } };
let $tt_v0: number;
const $tt_v1 = (key as any);
{
  const $tt_m = scr();
  switch ($tt_m.kind) {
    case "A": $tt_v0 = 0; break;
    case "B": $tt_v0 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
const obj = { [$tt_v1]: ($tt_v0 === 0 ? 1 : 2) };
console.log(log.join(", "), JSON.stringify(obj));
log.length = 0;
const part = { toString() { log.push("part->string"); return "x"; } };
let $tt_v2: number;
const $tt_v3 = (`${part}`);
{
  const $tt_m = scr();
  switch ($tt_m.kind) {
    case "A": $tt_v2 = 0; break;
    case "B": $tt_v2 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
const s = `${$tt_v3}${($tt_v2 === 0 ? 1 : 2)}`;
console.log(log.join(", "), s);
log.length = 0;
let $tt_v4: number;
const $tt_v5 = (`${{ toString() { log.push("inline->string"); return "y"; } }}`);
{
  const $tt_m = scr();
  switch ($tt_m.kind) {
    case "A": $tt_v4 = 0; break;
    case "B": $tt_v4 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
const inline = `${$tt_v5}${($tt_v4 === 0 ? 1 : 2)}`;
console.log(log.join(", "), inline);
log.length = 0;
let $tt_v6: number;
{
  const $tt_m = scr();
  switch ($tt_m.kind) {
    case "A": $tt_v6 = 0; break;
    case "B": $tt_v6 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
const plain = `${{ a: 1 }}${[1, 2]}${($tt_v6 === 0 ? 1 : 2)}`;
console.log(log.join(", "), plain);
