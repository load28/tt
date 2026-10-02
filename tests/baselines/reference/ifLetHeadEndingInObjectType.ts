//// [ifLetHeadEndingInObjectType.tt] ////
// An `if let` body opens at the first `{` after a complete bound
// expression, so a `{` TypeScript reads as part of that expression (an
// object type after `as` or `satisfies`, an object literal) does not end it.
type Maybe = { kind: "Some"; value: number } | { kind: "None" };
const values: unknown[] = [{ kind: "Some", value: 1 }, { kind: "None" }];
function read(x: unknown, y: unknown): string {
  if let Some(value: v) = x as { kind: "Some"; value: number } | { kind: "None" } {
    return `as ${v}`;
  } else if let Some(value: w) = y as Maybe & { kind: string } {
    return `intersection ${w}`;
  }
  if let Some(value: s) = { kind: "Some" as const, value: 3 } satisfies { kind: "Some"; value: number } {
    return `satisfies ${s}`;
  }
  return "none";
}
function literal(): number {
  if let Some(value: n) = { kind: "Some" as const, value: 4 } {
    return n;
  }
  return 0;
}
console.log(read(values[0], values[1]), read(values[1], values[0]), read(values[1], values[1]), literal());


//// [ifLetHeadEndingInObjectType.ts]
// An `if let` body opens at the first `{` after a complete bound
// expression, so a `{` TypeScript reads as part of that expression (an
// object type after `as` or `satisfies`, an object literal) does not end it.
type Maybe = { kind: "Some"; value: number } | { kind: "None" };
const values: unknown[] = [{ kind: "Some", value: 1 }, { kind: "None" }];
function read(x: unknown, y: unknown): string {
  {
    const $tt_t0 = x as { kind: "Some"; value: number } | { kind: "None" };
    if ($tt_t0.kind === "Some") {
      const { value: v } = $tt_t0;
      return `as ${v}`;
    } else {
      const $tt_t1 = y as Maybe & { kind: string };
      if ($tt_t1.kind === "Some") {
        const { value: w } = $tt_t1;
        return `intersection ${w}`;
      }
    }
  }
  {
    const $tt_t2 = { kind: "Some" as const, value: 3 } satisfies { kind: "Some"; value: number };
    if ($tt_t2.kind === "Some") {
      const { value: s } = $tt_t2;
      return `satisfies ${s}`;
    }
  }
  return "none";
}
function literal(): number {
  {
    const $tt_t3 = { kind: "Some" as const, value: 4 };
    if ($tt_t3.kind === "Some") {
      const { value: n } = $tt_t3;
      return n;
    }
  }
  return 0;
}
console.log(read(values[0], values[1]), read(values[1], values[0]), read(values[1], values[1]), literal());
