//// [main.tt] ////
export {};
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
type O = { kind: "Some"; value: number } | { kind: "None" };
const log: string[] = [];
function rd(n: number): R {
  log.push(`rd ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `neg ${n}` };
}
function opt(n: number): O {
  return n !== 0 ? { kind: "Some", value: n } : { kind: "None" };
}
function ifLetTry(n: number) {
  return result {
    if let Some(value: b) = opt(n) {
      return try rd(b);
    }
    return 0;
  };
}
function ifLetMatch(n: number) {
  return result {
    const a = try rd(n);
    if let Some(value: b) = opt(a) {
      return match (b) { 1 => "one", _ => "many" };
    }
    return "none";
  };
}
function letElseTry(n: number) {
  return result {
    const Some(value: b) = opt(n) else { return try rd(-1); };
    return b;
  };
}
function nestedIf(n: number) {
  return result {
    if let Some(value: b) = opt(n) {
      if (b > 1) return try rd(b);
    }
    return 0;
  };
}
function template(n: number) {
  return result {
    const a = try rd(n);
    if let Some(value: b) = opt(a) {
      return `<${match (b) { 1 => "one", _ => "many" }}>`;
    }
    return "";
  };
}
for (const n of [0, 1, 2, -3]) {
  console.log(n, JSON.stringify([ifLetTry(n), ifLetMatch(n), letElseTry(n), nestedIf(n), template(n)]), log.splice(0).join());
}

//// [twin.ts] ////
export {};
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
type O = { kind: "Some"; value: number } | { kind: "None" };
const log: string[] = [];
function rd(n: number): R {
  log.push(`rd ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `neg ${n}` };
}
function opt(n: number): O {
  return n !== 0 ? { kind: "Some", value: n } : { kind: "None" };
}
function ifLetTry(n: number) {
  const o = opt(n);
  if (o.kind === "Some") {
    const r = rd(o.value);
    if (!("value" in r)) return r;
    return { kind: "Ok" as const, value: r.value };
  }
  return { kind: "Ok" as const, value: 0 };
}
function ifLetMatch(n: number) {
  const r = rd(n);
  if (!("value" in r)) return r;
  const o = opt(r.value);
  if (o.kind === "Some") {
    return { kind: "Ok" as const, value: o.value === 1 ? "one" : "many" };
  }
  return { kind: "Ok" as const, value: "none" };
}
function letElseTry(n: number) {
  const o = opt(n);
  if (o.kind !== "Some") {
    const r = rd(-1);
    if (!("value" in r)) return r;
    return { kind: "Ok" as const, value: r.value };
  }
  return { kind: "Ok" as const, value: o.value };
}
function nestedIf(n: number) {
  const o = opt(n);
  if (o.kind === "Some") {
    if (o.value > 1) {
      const r = rd(o.value);
      if (!("value" in r)) return r;
      return { kind: "Ok" as const, value: r.value };
    }
  }
  return { kind: "Ok" as const, value: 0 };
}
function template(n: number) {
  const r = rd(n);
  if (!("value" in r)) return r;
  const o = opt(r.value);
  if (o.kind === "Some") {
    return { kind: "Ok" as const, value: `<${o.value === 1 ? "one" : "many"}>` };
  }
  return { kind: "Ok" as const, value: "" };
}
for (const n of [0, 1, 2, -3]) {
  console.log(n, JSON.stringify([ifLetTry(n), ifLetMatch(n), letElseTry(n), nestedIf(n), template(n)]), log.splice(0).join());
}


//// [main.ts]
export {};
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
type O = { kind: "Some"; value: number } | { kind: "None" };
const log: string[] = [];
function rd(n: number): R {
  log.push(`rd ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `neg ${n}` };
}
function opt(n: number): O {
  return n !== 0 ? { kind: "Some", value: n } : { kind: "None" };
}
function ifLetTry(n: number) {
  let $tt_v0: {
    kind: "Err";
    error: string;
} | {
    kind: "Ok";
    value: number;
};
  $tt_v0: {
    {
      const $tt_t0 = opt(n);
      if ($tt_t0.kind === "Some") {
        const { value: b } = $tt_t0;
        const $tt_t1 = rd(b);
        if (!("value" in $tt_t1)) {
          $tt_v0 = $tt_t1;
          break $tt_v0;
        }
        $tt_v0 = { kind: "Ok" as const, value: $tt_t1.value };
        break $tt_v0;
      }
    }
    {
      $tt_v0 = { kind: "Ok" as const, value: 0 };
      break $tt_v0;
    }
  }
  return $tt_v0;
}
function ifLetMatch(n: number) {
  let $tt_v1: {
    kind: "Err";
    error: string;
} | {
    kind: "Ok";
    value: string;
};
  $tt_v1: {
    const $tt_t2 = rd(n);
    if (!("value" in $tt_t2)) {
      $tt_v1 = $tt_t2;
      break $tt_v1;
    }
    const a = $tt_t2.value;
    {
      const $tt_t3 = opt(a);
      if ($tt_t3.kind === "Some") {
        const { value: b } = $tt_t3;
        {
          const $tt_m = b;
          switch ($tt_m) {
            case 1: {
              $tt_v1 = { kind: "Ok" as const, value: "one" };
              break;
            }
            default: {
              $tt_v1 = { kind: "Ok" as const, value: "many" };
              break;
            }
          }
        }
        break $tt_v1;
      }
    }
    {
      $tt_v1 = { kind: "Ok" as const, value: "none" };
      break $tt_v1;
    }
  }
  return $tt_v1;
}
function letElseTry(n: number) {
  let $tt_v2: {
    kind: "Err";
    error: string;
} | {
    kind: "Ok";
    value: number;
};
  $tt_v2: {
    const $tt_t4 = opt(n);
    if ($tt_t4.kind !== "Some") {
      const $tt_t5 = rd(-1);
      if (!("value" in $tt_t5)) {
        $tt_v2 = $tt_t5;
        break $tt_v2;
      }
      $tt_v2 = { kind: "Ok" as const, value: $tt_t5.value };
      break $tt_v2;
    }
    const { value: b } = $tt_t4;
    {
      $tt_v2 = { kind: "Ok" as const, value: b };
      break $tt_v2;
    }
  }
  return $tt_v2;
}
function nestedIf(n: number) {
  let $tt_v3: {
    kind: "Err";
    error: string;
} | {
    kind: "Ok";
    value: number;
};
  $tt_v3: {
    {
      const $tt_t6 = opt(n);
      if ($tt_t6.kind === "Some") {
        const { value: b } = $tt_t6;
        if (b > 1) {
          const $tt_t7 = rd(b);
          if (!("value" in $tt_t7)) {
            $tt_v3 = $tt_t7;
            break $tt_v3;
          }
          $tt_v3 = { kind: "Ok" as const, value: $tt_t7.value };
          break $tt_v3;
        }
      }
    }
    {
      $tt_v3 = { kind: "Ok" as const, value: 0 };
      break $tt_v3;
    }
  }
  return $tt_v3;
}
function template(n: number) {
  let $tt_v4: {
    kind: "Err";
    error: string;
} | {
    kind: "Ok";
    value: string;
};
  $tt_v4: {
    const $tt_t8 = rd(n);
    if (!("value" in $tt_t8)) {
      $tt_v4 = $tt_t8;
      break $tt_v4;
    }
    const a = $tt_t8.value;
    {
      const $tt_t9 = opt(a);
      if ($tt_t9.kind === "Some") {
        const { value: b } = $tt_t9;
        let $tt_v9: string;
        {
          const $tt_m = b;
          switch ($tt_m) {
            case 1: {
              $tt_v9 = "one";
              break;
            }
            default: {
              $tt_v9 = "many";
              break;
            }
          }
        }
        $tt_v4 = { kind: "Ok" as const, value: `<${$tt_v9}>` };
        break $tt_v4;
      }
    }
    {
      $tt_v4 = { kind: "Ok" as const, value: "" };
      break $tt_v4;
    }
  }
  return $tt_v4;
}
for (const n of [0, 1, 2, -3]) {
  console.log(n, JSON.stringify([ifLetTry(n), ifLetMatch(n), letElseTry(n), nestedIf(n), template(n)]), log.splice(0).join());
}
\ No newline at end of file

//// [twin.ts]
export {};
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
type O = { kind: "Some"; value: number } | { kind: "None" };
const log: string[] = [];
function rd(n: number): R {
  log.push(`rd ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `neg ${n}` };
}
function opt(n: number): O {
  return n !== 0 ? { kind: "Some", value: n } : { kind: "None" };
}
function ifLetTry(n: number) {
  const o = opt(n);
  if (o.kind === "Some") {
    const r = rd(o.value);
    if (!("value" in r)) return r;
    return { kind: "Ok" as const, value: r.value };
  }
  return { kind: "Ok" as const, value: 0 };
}
function ifLetMatch(n: number) {
  const r = rd(n);
  if (!("value" in r)) return r;
  const o = opt(r.value);
  if (o.kind === "Some") {
    return { kind: "Ok" as const, value: o.value === 1 ? "one" : "many" };
  }
  return { kind: "Ok" as const, value: "none" };
}
function letElseTry(n: number) {
  const o = opt(n);
  if (o.kind !== "Some") {
    const r = rd(-1);
    if (!("value" in r)) return r;
    return { kind: "Ok" as const, value: r.value };
  }
  return { kind: "Ok" as const, value: o.value };
}
function nestedIf(n: number) {
  const o = opt(n);
  if (o.kind === "Some") {
    if (o.value > 1) {
      const r = rd(o.value);
      if (!("value" in r)) return r;
      return { kind: "Ok" as const, value: r.value };
    }
  }
  return { kind: "Ok" as const, value: 0 };
}
function template(n: number) {
  const r = rd(n);
  if (!("value" in r)) return r;
  const o = opt(r.value);
  if (o.kind === "Some") {
    return { kind: "Ok" as const, value: `<${o.value === 1 ? "one" : "many"}>` };
  }
  return { kind: "Ok" as const, value: "" };
}
for (const n of [0, 1, 2, -3]) {
  console.log(n, JSON.stringify([ifLetTry(n), ifLetMatch(n), letElseTry(n), nestedIf(n), template(n)]), log.splice(0).join());
}
