//// [jsxValueInsideIfLetBodyGetsItsOwnHostRewrite.ttx] ////
variant E { A(value: string), B }
const view = (node: E) => {
  if let A(value) = node {
    return <section data-kind={match (node) { A => "a", B => "b" }}>{value |> .trim()}</section>;
  } else {
    return null;
  }
};


//// [jsxValueInsideIfLetBodyGetsItsOwnHostRewrite.tsx]
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
type E =
  | { kind: "A"; value: string }
  | { kind: "B" };
const E = {
  A: (value: string): E => ({ kind: "A", value }),
  B: { kind: "B" } as const,
};
const view = (node: E) => {
  {
    const $tt_t0 = node;
    if ($tt_t0.kind === "A") {
      const { value } = $tt_t0;
      let $tt_v0: string;
      {
        const $tt_m = node;
        switch ($tt_m.kind) {
          case "A": {
            $tt_v0 = "a";
            break;
          }
          case "B": {
            $tt_v0 = "b";
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      return <section data-kind={$tt_v0}>{value.trim()}</section>;
    } else {
      return null;
    }
  }
};
