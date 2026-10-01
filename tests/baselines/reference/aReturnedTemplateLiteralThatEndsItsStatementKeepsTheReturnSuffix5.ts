//// [aReturnedTemplateLiteralThatEndsItsStatementKeepsTheReturnSuffix5.tt] ////
variant O { A(n: number), B }
declare const o: O;
declare function tag(s: TemplateStringsArray, ...v: unknown[]): string;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
export const a = match (o) { A(n) => { return `${n}`}, B => 0 };


//// [aReturnedTemplateLiteralThatEndsItsStatementKeepsTheReturnSuffix5.ts]
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
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const o: O;
declare function tag(s: TemplateStringsArray, ...v: unknown[]): string;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
let $tt_v0: (string) | (number);
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "A": {
      const { n } = $tt_m;
      $tt_v0 = `${n}`; break;
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
export const a = $tt_v0;
