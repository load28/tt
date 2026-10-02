//// [letElseIfLetDivergence.tt] ////
// let-else and if-let evaluate their initializer once, bind on a match, and
// leave through the else block's own exit (return, continue, break, throw).
variant Found { Hit(value: number), Near(value: number), Miss }
const log: string[] = [];
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
function lookup(key: string): Found {
  log.push(`lookup ${key}`);
  return key === "a" ? Found.Hit(1) : key === "b" ? Found.Near(2) : Found.Miss;
}
function viaReturn(key: string): string {
  const Hit(value) = lookup(key) else { return `miss ${key}`; };
  log.push("bound");
  return `hit ${value}`;
}
flush("return hit", viaReturn("a"));
flush("return miss", viaReturn("z"));
function viaContinue(keys: string[]): number[] {
  const seen: number[] = [];
  for (const key of keys) {
    const Hit(value) | Near(value) = lookup(key) else { continue; };
    seen.push(value);
  }
  return seen;
}
flush("continue", viaContinue(["a", "z", "b", "a"]));
function viaBreak(keys: string[]): number[] {
  const seen: number[] = [];
  outer: for (const key of keys) {
    for (;;) {
      const Hit(value) = lookup(key) else { break outer; };
      seen.push(value);
      break;
    }
  }
  return seen;
}
flush("labeled break", viaBreak(["a", "a", "b", "a"]));
function viaThrow(key: string): number {
  const Near(value) = lookup(key) else { throw new Error(`not near: ${key}`); };
  return value;
}
for (const key of ["b", "a"]) {
  try {
    flush(`throw ${key}`, viaThrow(key));
  } catch (e) {
    flush(`throw ${key}`, (e as Error).message);
  }
}
function describe(first: Found, second: Found): string {
  if let Hit(value) = (log.push("first"), first) {
    return `first hit ${value}`;
  } else if let Hit(value: v) | Near(value: v) = (log.push("second"), second) {
    return `second ${v}`;
  } else {
    log.push("else");
  }
  return "neither";
}
flush("if let first", describe(Found.Hit(1), Found.Miss));
flush("if let second", describe(Found.Miss, Found.Near(5)));
flush("if let neither", describe(Found.Miss, Found.Miss));
function asserted(n: number): number {
  const Hit(value) = ({ kind: "Hit", value: n }) as const else { return -1; };
  if let Hit(value: doubled) = ({ kind: "Hit", value: value * 2 }) as const {
    return doubled;
  }
  return value;
}
flush("as const initializers", asserted(4));
let scoped = "outer";
if let Hit(value: scoped) = Found.Hit(9) {
  log.push(`inner scoped ${scoped}`);
}
flush("binding scope", scoped);
export {};


//// [letElseIfLetDivergence.ts]
// let-else and if-let evaluate their initializer once, bind on a match, and
// leave through the else block's own exit (return, continue, break, throw).
type Found =
  | { kind: "Hit"; value: number }
  | { kind: "Near"; value: number }
  | { kind: "Miss" };
const Found = {
  Hit: (value: number): Found => ({ kind: "Hit", value }),
  Near: (value: number): Found => ({ kind: "Near", value }),
  Miss: { kind: "Miss" } as const,
};
const log: string[] = [];
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
function lookup(key: string): Found {
  log.push(`lookup ${key}`);
  return key === "a" ? Found.Hit(1) : key === "b" ? Found.Near(2) : Found.Miss;
}
function viaReturn(key: string): string {
  const $tt_t0 = lookup(key);
  if ($tt_t0.kind !== "Hit") {
    return `miss ${key}`;
  }
  const { value } = $tt_t0;
  log.push("bound");
  return `hit ${value}`;
}
flush("return hit", viaReturn("a"));
flush("return miss", viaReturn("z"));
function viaContinue(keys: string[]): number[] {
  const seen: number[] = [];
  for (const key of keys) {
    const $tt_t1 = lookup(key);
    if ($tt_t1.kind !== "Hit" && $tt_t1.kind !== "Near") {
      continue;
    }
    const { value } = $tt_t1;
    seen.push(value);
  }
  return seen;
}
flush("continue", viaContinue(["a", "z", "b", "a"]));
function viaBreak(keys: string[]): number[] {
  const seen: number[] = [];
  outer: for (const key of keys) {
    for (;;) {
      const $tt_t2 = lookup(key);
      if ($tt_t2.kind !== "Hit") {
        break outer;
      }
      const { value } = $tt_t2;
      seen.push(value);
      break;
    }
  }
  return seen;
}
flush("labeled break", viaBreak(["a", "a", "b", "a"]));
function viaThrow(key: string): number {
  const $tt_t3 = lookup(key);
  if ($tt_t3.kind !== "Near") {
    throw new Error(`not near: ${key}`);
  }
  const { value } = $tt_t3;
  return value;
}
for (const key of ["b", "a"]) {
  try {
    flush(`throw ${key}`, viaThrow(key));
  } catch (e) {
    flush(`throw ${key}`, (e as Error).message);
  }
}
function describe(first: Found, second: Found): string {
  {
    const $tt_t4 = (log.push("first"), first);
    if ($tt_t4.kind === "Hit") {
      const { value } = $tt_t4;
      return `first hit ${value}`;
    } else {
      const $tt_t5 = (log.push("second"), second);
      if ($tt_t5.kind === "Hit" || $tt_t5.kind === "Near") {
        const { value: v } = $tt_t5;
        return `second ${v}`;
      } else {
        log.push("else");
      }
    }
  }
  return "neither";
}
flush("if let first", describe(Found.Hit(1), Found.Miss));
flush("if let second", describe(Found.Miss, Found.Near(5)));
flush("if let neither", describe(Found.Miss, Found.Miss));
function asserted(n: number): number {
  const $tt_t6 = ({ kind: "Hit", value: n }) as const;
  if ($tt_t6.kind !== "Hit") {
    return -1;
  }
  const { value } = $tt_t6;
  {
    const $tt_t7 = ({ kind: "Hit", value: value * 2 }) as const;
    if ($tt_t7.kind === "Hit") {
      const { value: doubled } = $tt_t7;
      return doubled;
    }
  }
  return value;
}
flush("as const initializers", asserted(4));
let scoped = "outer";
{
  const $tt_t8 = Found.Hit(9);
  if ($tt_t8.kind === "Hit") {
    const { value: scoped } = $tt_t8;
    log.push(`inner scoped ${scoped}`);
  }
}
flush("binding scope", scoped);
export {};
