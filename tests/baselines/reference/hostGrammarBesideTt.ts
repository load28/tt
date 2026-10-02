//// [hostGrammarBesideTt.tt] ////
// Repro from TASK-641
export variant Flag { On, Off }
declare const dec: any;
declare const readonly: unknown;
export @dec abstract class Base {}
export const n = (readonly as number);
export const label = (f: Flag) => match (f) {
  On => <string>"on",
  Off => "off",
};
export { type Flag as "flag type" };


//// [hostGrammarBesideTt.ts]
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
// Repro from TASK-641
export type Flag =
  | { kind: "On" }
  | { kind: "Off" };
export const Flag = {
  On: { kind: "On" } as const,
  Off: { kind: "Off" } as const,
};
declare const dec: any;
declare const readonly: unknown;
export @dec abstract class Base {}
export const n = (readonly as number);
export const label = (f: Flag) => {
  let $tt_v0: string;
  {
    const $tt_m = f;
    switch ($tt_m.kind) {
      case "On": {
        $tt_v0 = <string>"on";
        break;
      }
      case "Off": {
        $tt_v0 = "off";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};
export { type Flag as "flag type" };
