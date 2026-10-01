//// [main.tt] ////
export {};
type O = { kind: "Some"; value: number } | { kind: "None" };
function opt(n: number): O {
  return n !== 0 ? { kind: "Some", value: n } : { kind: "None" };
}
function whole(k: number, n: number): string {
  const v = match (k) {
    1 => {
      if let Some(value: b) = opt(n) {
        return match (b) { 1 => "one", _ => "many" };
      }
      return "none";
    },
    _ => "other",
  };
  return v;
}
function nested(k: number, n: number): string {
  const v = match (k) {
    1 => {
      if let Some(value: b) = opt(n) {
        if (b > 0) return match (b) { 1 => "one", _ => "many" };
      }
      return "none";
    },
    _ => "other",
  };
  return v;
}
function template(k: number, n: number): string {
  const v = match (k) {
    1 => {
      if let Some(value: b) = opt(n) {
        return `<${match (b) { 1 => "one", _ => "many" }}>`;
      }
      return "none";
    },
    _ => "other",
  };
  return v;
}
for (const [k, n] of [[1, 1], [1, 2], [1, -2], [1, 0], [2, 1]]) {
  console.log(k, n, whole(k, n), nested(k, n), template(k, n));
}

//// [twin.ts] ////
export {};
type O = { kind: "Some"; value: number } | { kind: "None" };
function opt(n: number): O {
  return n !== 0 ? { kind: "Some", value: n } : { kind: "None" };
}
function arm(n: number, wrap: (s: string) => string, positiveOnly: boolean): string {
  const o = opt(n);
  if (o.kind === "Some") {
    if (!positiveOnly || o.value > 0) return wrap(o.value === 1 ? "one" : "many");
  }
  return "none";
}
function whole(k: number, n: number): string {
  return k === 1 ? arm(n, (s) => s, false) : "other";
}
function nested(k: number, n: number): string {
  return k === 1 ? arm(n, (s) => s, true) : "other";
}
function template(k: number, n: number): string {
  return k === 1 ? arm(n, (s) => `<${s}>`, false) : "other";
}
for (const [k, n] of [[1, 1], [1, 2], [1, -2], [1, 0], [2, 1]]) {
  console.log(k, n, whole(k, n), nested(k, n), template(k, n));
}


//// [main.ts]
export {};
type O = { kind: "Some"; value: number } | { kind: "None" };
function opt(n: number): O {
  return n !== 0 ? { kind: "Some", value: n } : { kind: "None" };
}
function whole(k: number, n: number): string {
  let $tt_v0: string;
  {
    const $tt_m = k;
    switch ($tt_m) {
      case 1: {
        {
          const $tt_t0 = opt(n);
          if ($tt_t0.kind === "Some") {
            const { value: b } = $tt_t0;
            {
              const $tt_m = b;
              switch ($tt_m) {
                case 1: {
                  $tt_v0 = "one";
                  break;
                }
                default: {
                  $tt_v0 = "many";
                  break;
                }
              }
            }
            break;
          }
        }
        $tt_v0 = "none";
        break;
      }
      default: {
        $tt_v0 = "other";
        break;
      }
    }
  }
  const v = $tt_v0;
  return v;
}
function nested(k: number, n: number): string {
  let $tt_v1: string;
  {
    const $tt_m = k;
    switch ($tt_m) {
      case 1: {
        {
          const $tt_t1 = opt(n);
          if ($tt_t1.kind === "Some") {
            const { value: b } = $tt_t1;
            if (b > 0) { {
              const $tt_m = b;
              switch ($tt_m) {
                case 1: {
                  $tt_v1 = "one";
                  break;
                }
                default: {
                  $tt_v1 = "many";
                  break;
                }
              }
            }
            break; }
          }
        }
        $tt_v1 = "none";
        break;
      }
      default: {
        $tt_v1 = "other";
        break;
      }
    }
  }
  const v = $tt_v1;
  return v;
}
function template(k: number, n: number): string {
  let $tt_v2: string;
  {
    const $tt_m = k;
    switch ($tt_m) {
      case 1: {
        {
          const $tt_t2 = opt(n);
          if ($tt_t2.kind === "Some") {
            const { value: b } = $tt_t2;
            let $tt_v5: string;
            {
              const $tt_m = b;
              switch ($tt_m) {
                case 1: {
                  $tt_v5 = "one";
                  break;
                }
                default: {
                  $tt_v5 = "many";
                  break;
                }
              }
            }
            $tt_v2 = `<${$tt_v5}>`;
            break;
          }
        }
        $tt_v2 = "none";
        break;
      }
      default: {
        $tt_v2 = "other";
        break;
      }
    }
  }
  const v = $tt_v2;
  return v;
}
for (const [k, n] of [[1, 1], [1, 2], [1, -2], [1, 0], [2, 1]]) {
  console.log(k, n, whole(k, n), nested(k, n), template(k, n));
}
\ No newline at end of file

//// [twin.ts]
export {};
type O = { kind: "Some"; value: number } | { kind: "None" };
function opt(n: number): O {
  return n !== 0 ? { kind: "Some", value: n } : { kind: "None" };
}
function arm(n: number, wrap: (s: string) => string, positiveOnly: boolean): string {
  const o = opt(n);
  if (o.kind === "Some") {
    if (!positiveOnly || o.value > 0) return wrap(o.value === 1 ? "one" : "many");
  }
  return "none";
}
function whole(k: number, n: number): string {
  return k === 1 ? arm(n, (s) => s, false) : "other";
}
function nested(k: number, n: number): string {
  return k === 1 ? arm(n, (s) => s, true) : "other";
}
function template(k: number, n: number): string {
  return k === 1 ? arm(n, (s) => `<${s}>`, false) : "other";
}
for (const [k, n] of [[1, 1], [1, 2], [1, -2], [1, 0], [2, 1]]) {
  console.log(k, n, whole(k, n), nested(k, n), template(k, n));
}
