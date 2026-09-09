type Inner =
  | { kind: "Yes"; n: number }
  | { kind: "No" };
const Inner = {
  Yes: (n: number): Inner => ({ kind: "Yes", n }),
  No: { kind: "No" } as const,
};
type Outer =
  | { kind: "Wrap"; inner: Inner }
  | { kind: "Bare" };
const Outer = {
  Wrap: (inner: Inner): Outer => ({ kind: "Wrap", inner }),
  Bare: { kind: "Bare" } as const,
};
declare const o: Outer;

let $tt_v0;
{
  const $tt_m = o;
  do {
    if ($tt_m.kind === "Wrap" && $tt_m.inner.kind === "Yes") {
      const { n } = $tt_m.inner;
      $tt_v0 = n;
      break;
    }
    if ($tt_m.kind === "Wrap" && $tt_m.inner.kind === "No") {
      $tt_v0 = 0;
      break;
    }
    if ($tt_m.kind === "Bare") {
      $tt_v0 = -1;
      break;
    }
    throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
  } while (false);
}
export const value = $tt_v0;
