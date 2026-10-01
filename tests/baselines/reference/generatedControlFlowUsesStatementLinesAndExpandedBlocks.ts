//// [generatedControlFlowUsesStatementLinesAndExpandedBlocks.tt] ////
variant E { A(v: number), B }
function f(e: E): Result<number, string> {
  const value = try read();
  const matched = match (e) { A(v) => v, B => 0 };
  return Result.Ok(value + matched);
}
function block(e: E): number {
  return match (e) {
    A(v) if v > 0 => {
      const doubled = v * 2;
      return doubled;
    },
    _ => 0,
  };
}
function bind(e: E): number {
  const A(v) = e else {
    return 0;
  };
  return v;
}
const computed = result {
  return try read();
};


//// [generatedControlFlowUsesStatementLinesAndExpandedBlocks.ts]
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
  | { kind: "A"; v: number }
  | { kind: "B" };
const E = {
  A: (v: number): E => ({ kind: "A", v }),
  B: { kind: "B" } as const,
};
function f(e: E): Result<number, string> {
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const value = $tt_t0.value;
  let $tt_v0: number;
  {
    const $tt_m = e;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        $tt_v0 = v;
        break;
      }
      case "B": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const matched = $tt_v0;
  return Result.Ok(value + matched);
}
function block(e: E): number {
  let $tt_v1: number;
  {
    const $tt_m = e;
    do {
      if ($tt_m.kind === "A") {
        const { v } = $tt_m;
        if (v > 0) {
          {
      const doubled = v * 2;
      $tt_v1 = doubled;
      break;
          }
        }
      }
      $tt_v1 = 0;
      break;
    } while (false);
  }
  return $tt_v1;
}
function bind(e: E): number {
  const $tt_t1 = e;
  if ($tt_t1.kind !== "A") {
    return 0;
  }
  const { v } = $tt_t1;
  return v;
}
let $tt_v2$computed;
$tt_v2$computed: {
  const $tt_t2 = read();
  if (!("value" in $tt_t2)) {
    $tt_v2$computed = $tt_t2;
    break $tt_v2$computed;
  }
  const $tt_a0 = { value: { kind: "Ok" as const, value: $tt_t2.value } };
  $tt_v2$computed = $tt_a0.value;
  break $tt_v2$computed;
}
const computed = $tt_v2$computed;
