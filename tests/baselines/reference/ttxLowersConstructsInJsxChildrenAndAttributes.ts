//// [ttxLowersConstructsInJsxChildrenAndAttributes.ttx] ////
variant State { Ready(value: string), Empty }
declare const state: State;
const child = <section>{match (state) {
  Ready(value) => <strong>{value}</strong>,
  Empty => <span>empty</span>,
}}</section>;
const prop = <Panel before={mark("before")} render={() => match (state) {
  Ready(value) => <strong>{value}</strong>,
  Empty => null,
}} after={mark("after")} />;
const ordered = (state: State) => <Panel before={mark("first")} value={match (state) {
  Ready(value) => value,
  Empty => "",
}} after={mark("last")} />;


//// [ttxLowersConstructsInJsxChildrenAndAttributes.tsx]
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
let $tt_v0$child;
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v0$child = <strong>{value}</strong>;
      break;
    }
    case "Empty": {
      $tt_v0$child = <span>empty</span>;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const child = <section>{$tt_v0$child}</section>;
const prop = <Panel before={mark("before")} render={() => {
  let $tt_v1;
  {
    const $tt_m = state;
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v1 = <strong>{value}</strong>;
        break;
      }
      case "Empty": {
        $tt_v1 = null;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v1;
}} after={mark("after")} />;
const ordered = (state: State) => {
  let $tt_v2: string;
  const $tt_v3 = (Panel);
  const $tt_v4 = (mark("first"));
  {
    const $tt_m = state;
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v2 = value;
        break;
      }
      case "Empty": {
        $tt_v2 = "";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return <$tt_v3 before={$tt_v4} value={$tt_v2} after={mark("last")} />;
};
