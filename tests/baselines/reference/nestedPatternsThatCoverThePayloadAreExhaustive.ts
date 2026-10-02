//// [nestedPatternsThatCoverThePayloadAreExhaustive.tt] ////
variant Inner { Yes(n: number), No }
variant Outer { Wrap(inner: Inner), Bare }
const a = match (o) {
  Wrap(inner: Yes(n)) => n,
  Wrap(inner: No()) => 0,
  Bare => -1,
};


//// [nestedPatternsThatCoverThePayloadAreExhaustive.ts]
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
let $tt_v0$a;
{
  const $tt_m = o;
  do {
    if ($tt_m.kind === "Wrap" && $tt_m.inner.kind === "Yes") {
      const { n } = $tt_m.inner;
      $tt_v0$a = n;
      break;
    }
    if ($tt_m.kind === "Wrap" && $tt_m.inner.kind === "No") {
      $tt_v0$a = 0;
      break;
    }
    if ($tt_m.kind === "Bare") {
      $tt_v0$a = -1;
      break;
    }
    throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  } while (false);
}
const a = $tt_v0$a;
