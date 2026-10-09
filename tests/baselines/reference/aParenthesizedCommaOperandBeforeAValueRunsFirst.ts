//// [aParenthesizedCommaOperandBeforeAValueRunsFirst.tt] ////
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const log: string[] = [];
const note = (s: string) => { log.push(s); return 0; };
const r = (n: number): R<number> => { log.push("r"); return { kind: "Ok", value: n }; };
function f(o: number) { return ((note("o")), match (o) { 1 => "one", _ => "other" }); }
function h(): R<string> { const v = ((note("g")), try r(1)); return { kind: "Ok", value: String(v) }; }
console.log(f(1), JSON.stringify(h()), log.join(","));
export {};


//// [aParenthesizedCommaOperandBeforeAValueRunsFirst.ts]
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const log: string[] = [];
const note = (s: string) => { log.push(s); return 0; };
const r = (n: number): R<number> => { log.push("r"); return { kind: "Ok", value: n }; };
function f(o: number) { let $tt_v0: number;
((note("o")));
{
  const $tt_m = o;
  switch ($tt_m) {
    case 1: $tt_v0 = 0; break;
    default: $tt_v0 = 1; break;
  }
}
return ( ($tt_v0 === 0 ? "one" : "other")); }
function h(): R<string> { let $tt_v2: number;
((note("g")));
const $tt_t0 = r(1);
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
$tt_v2 = $tt_t0.value;
const v = ( $tt_v2); return { kind: "Ok", value: String(v) }; }
console.log(f(1), JSON.stringify(h()), log.join(","));
export {};
