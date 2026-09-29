type State =
  | { kind: "Ready"; value: string }
  | { kind: "Empty" };
const State = {
  Ready: (value: string): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
declare const state: State;
declare const Panel: (props: { value: string }) => unknown;

let $tt_v0;
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v0 = <strong>{value}</strong>;
      break;
    }
    case "Empty": {
      $tt_v0 = <span>empty</span>;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
export const view = <main>
  {$tt_v0}
</main>;

let $tt_v1: string;
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v1 = value;
      break;
    }
    case "Empty": {
      $tt_v1 = "";
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
export const panel = <Panel value={$tt_v1} />;
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
