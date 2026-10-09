//// [literalMatchInEveryPosition.tt] ////
// Runtime companion of TASK-642: a literal match in the positions the
// mixed-pattern repro used dispatches on the value in each of them.
const seen: string[] = [];
function consume(n: number): void {
  seen.push(`consume ${n}`);
}
function current(on: boolean): boolean {
  seen.push(`read ${on}`);
  return on;
}
for (const on of [true, false]) {
  const flag = current(on);
  consume(match (flag) { true => 1, false => 2 });
  const held = [match (flag) { true => 1, _ => 2 }];
  seen.push(`held ${held.join(",")}`);
  const later = () => consume(match (flag) { true => 1, _ => 2 } + match (flag) { false => 3, true => 4 });
  later();
  console.log(seen.join(" | "));
  seen.length = 0;
}
const codes = [200, 201, 404, 500].map((code) => match (code) { 200 | 201 => "success", 404 => "not found", _ => "other" });
console.log(codes.join(" "));
export {};


//// [literalMatchInEveryPosition.ts]
function $tt_raise(error: unknown): never { throw error; }
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
// Runtime companion of TASK-642: a literal match in the positions the
// mixed-pattern repro used dispatches on the value in each of them.
const seen: string[] = [];
function consume(n: number): void {
  seen.push(`consume ${n}`);
}
function current(on: boolean): boolean {
  seen.push(`read ${on}`);
  return on;
}
for (const on of [true, false]) {
  const flag = current(on);
  let $tt_v0: number;
  const $tt_v1: typeof consume = (consume);
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: $tt_v0 = 0; break;
      case false: $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
  $tt_v1(($tt_v0 === 0 ? 1 : 2));
  let $tt_v2: number;
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: $tt_v2 = 0; break;
      default: $tt_v2 = 1; break;
    }
  }
  const held = [($tt_v2 === 0 ? 1 : 2)];
  seen.push(`held ${held.join(",")}`);
  const later = () => {
    let $tt_subject_2;
    let $tt_subject_3;
    
    return consume(($tt_subject_2 = flag, ($tt_subject_2 === true) ? 1 : 2) + ($tt_subject_3 = flag, ($tt_subject_3 === false) ? 3 : ($tt_subject_3 === true) ? 4 : $tt_raise(new Error("tt match: unexpected literal " + $tt_show($tt_subject_3)))));
  };
  later();
  console.log(seen.join(" | "));
  seen.length = 0;
}
const codes = [200, 201, 404, 500].map((code) => {
  let $tt_v6: string;
  {
    const $tt_m = code;
    switch ($tt_m) {
      case 200: case 201: {
        $tt_v6 = "success";
        break;
      }
      case 404: {
        $tt_v6 = "not found";
        break;
      }
      default: {
        $tt_v6 = "other";
        break;
      }
    }
  }
  return $tt_v6;
});
console.log(codes.join(" "));
export {};
