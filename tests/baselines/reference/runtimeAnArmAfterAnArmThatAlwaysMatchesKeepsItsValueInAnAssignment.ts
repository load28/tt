//// [runtimeAnArmAfterAnArmThatAlwaysMatchesKeepsItsValueInAnAssignment.tt] ////

variant Dir { Up, Down }
function pick(a: Dir, b: Dir): unknown {
  let v: unknown;
  v = match (a, b) { (Up, Down) => "up-down", (_, _) => "any", _ => "never" };
  return v;
}
const target = { value: 0 as unknown };
function assign(a: Dir, b: Dir): unknown {
  target.value = match (a, b) { (_, _) => 1, _ => 2 };
  return target.value;
}
console.log(pick(Dir.Up, Dir.Down), pick(Dir.Down, Dir.Up), assign(Dir.Up, Dir.Up));

export {};


//// [runtimeAnArmAfterAnArmThatAlwaysMatchesKeepsItsValueInAnAssignment.ts]

type Dir =
  | { kind: "Up" }
  | { kind: "Down" };
const Dir = {
  Up: { kind: "Up" } as const,
  Down: { kind: "Down" } as const,
};
function pick(a: Dir, b: Dir): unknown {
  let v: unknown;
  let $tt_v0: string;
  {
    const $tt_m0 = a;
    const $tt_m1 = b;
    do {
      if ($tt_m0.kind === "Up" && $tt_m1.kind === "Down") {
        $tt_v0 = "up-down";
        break;
      }
      $tt_v0 = "any";
      break;
      $tt_v0 = "never";
      break;
    } while (false);
  }
  v = $tt_v0;
  return v;
}
const target = { value: 0 as unknown };
function assign(a: Dir, b: Dir): unknown {
  let $tt_v1: number;
  {
    const $tt_m0 = a;
    const $tt_m1 = b;
    do {
      $tt_v1 = 1;
      break;
      $tt_v1 = 2;
      break;
    } while (false);
  }
  target.value = $tt_v1;
  return target.value;
}
console.log(pick(Dir.Up, Dir.Down), pick(Dir.Down, Dir.Up), assign(Dir.Up, Dir.Up));

export {};
