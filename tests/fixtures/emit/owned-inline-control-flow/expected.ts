type Opt =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const Opt = {
  Some: (value: number): Opt => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
export function pick(o: Opt): number {
  let $tt_v0: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "Some": {
        const { value } = $tt_m;
        {
          const $tt_t0 = o;
          if ($tt_t0.kind === "Some") {
            const { value: v2 } = $tt_t0;
            $tt_v0 = v2; break;
          }
        } $tt_v0 = value; break;
      }
      case "None": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
      }
    }
  }
  const v = $tt_v0;
  return v + 100;
}
export function read(o: Opt) {
  let $tt_v1: {
    kind: "Ok";
    value: number;
};
  $tt_v1: {
    let n;
    let $tt_v2: number;
    const $tt_t1 = ({ kind: "Ok" as const, value: pick(o) });
    if (!("value" in $tt_t1)) {
      $tt_v1 = $tt_t1;
      break $tt_v1;
    }
    $tt_v2 = $tt_t1.value;
    n = $tt_v2;
    {
      $tt_v1 = { kind: "Ok" as const, value: n };
      break $tt_v1;
    }
  }
  return $tt_v1;
}
export const strings = [1].map(x => {
let $tt_v3: number;
const $tt_v4 = (`${x}`);
{
  const $tt_m = Opt.Some(x);
  switch ($tt_m.kind) {
    case "Some": {
      const { value } = $tt_m;
      $tt_v3 = value;
      break;
    }
    case "None": {
      $tt_v3 = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
return $tt_v4 + $tt_v3;
});
let $tt_v5: number[];
{
  const $tt_m = Opt.Some(1);
  switch ($tt_m.kind) {
    case "Some": {
      const { value } = $tt_m;
      $tt_v5 = [value];
      break;
    }
    case "None": {
      $tt_v5 = [];
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
export const values = $tt_v5;
