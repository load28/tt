//// [runtimeGenericVariant.tt] ////

variant TOption<T> {
  Some(value: T),
  None,
}

function unwrapOr<T>(o: TOption<T>, fallback: T): T {
  return match (o) {
    Some(value) => value,
    None => fallback,
  };
}

console.log(unwrapOr(TOption.Some(7), 0));
console.log(unwrapOr<number>(TOption.None, 42));

export {};


//// [runtimeGenericVariant.ts]
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

type TOption<T> =
  | { kind: "Some"; value: T }
  | { kind: "None" };
const TOption = {
  Some: <T>(value: T): TOption<T> => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};

function unwrapOr<T>(o: TOption<T>, fallback: T): T {
  let $tt_v0: T;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "Some": {
        const { value } = $tt_m;
        $tt_v0 = value;
        break;
      }
      case "None": {
        $tt_v0 = fallback;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}

console.log(unwrapOr(TOption.Some(7), 0));
console.log(unwrapOr<number>(TOption.None, 42));

export {};
