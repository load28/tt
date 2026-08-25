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
    case "Ready": { const { value } = $tt_m; $tt_v0 = <strong>{value}</strong>; break; }
    case "Empty": { $tt_v0 = <span>empty</span>; break; }
    default: { throw new Error("tt match: unexpected case " + JSON.stringify($tt_m)); }
  }
}
export const view = <main>
  {$tt_v0}
</main>;

let $tt_v1;
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": { const { value } = $tt_m; $tt_v1 = value; break; }
    case "Empty": { $tt_v1 = ""; break; }
    default: { throw new Error("tt match: unexpected case " + JSON.stringify($tt_m)); }
  }
}
export const panel = <Panel value={$tt_v1} />;
