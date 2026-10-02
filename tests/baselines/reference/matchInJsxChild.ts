//// [matchInJsxChild.ttx] ////
declare global {
  namespace JSX { interface IntrinsicElements { main: {}; b: {}; } }
}

variant State { Ready(value: string), Empty }
export const render = (state: State) => <main>{match (state) {
  Ready(value) => <b>{value}</b>,
  Empty => null,
}}</main>;


//// [matchInJsxChild.tsx]
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
declare global {
  namespace JSX { interface IntrinsicElements { main: {}; b: {}; } }
}

type State =
  | { kind: "Ready"; value: string }
  | { kind: "Empty" };
const State = {
  Ready: (value: string): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
export const render = (state: State) => {
  let $tt_v0;
  {
    const $tt_m = state;
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v0 = <b>{value}</b>;
        break;
      }
      case "Empty": {
        $tt_v0 = null;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return <main>{$tt_v0}</main>;
};
