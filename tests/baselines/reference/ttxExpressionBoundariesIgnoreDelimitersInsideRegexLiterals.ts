//// [ttxExpressionBoundariesIgnoreDelimitersInsideRegexLiterals.ttx] ////
variant State { Ready(value: string), Empty }
declare const state: State;
const view = <Panel visible={/}/.test("}")} value={match (state) {
  Ready(value) => value,
  Empty => "",
}} />;


//// [ttxExpressionBoundariesIgnoreDelimitersInsideRegexLiterals.tsx]
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
type State =
  | { kind: "Ready"; value: string }
  | { kind: "Empty" };
const State = {
  Ready: (value: string): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
declare const state: State;
let $tt_v0$view: string;
const $tt_v1$view = (/}/.test("}"));
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v0$view = value;
      break;
    }
    case "Empty": {
      $tt_v0$view = "";
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const view = <Panel visible={$tt_v1$view} value={$tt_v0$view} />;
