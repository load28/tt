//// [aHoistedValueInAMemberStepRunsAfterThePipedValueAndItsMethod.tt] ////

variant E { A(value: number), B }
type N = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
type R = { kind: "Ok"; value: Box } | { kind: "Err"; error: string };
const order: string[] = [];
const mark = <T,>(name: string, value: T): T => { order.push(name); return value; };
const subject = (name: string): E => { order.push(name); return E.A(1); };
class Box {
  constructor(readonly n: number) {}
  get add() { order.push("get"); return Box.prototype.addTo; }
  addTo(amount: number): Box { order.push("call"); return new Box(this.n + amount); }
}
const factory = {
  get make() { order.push("get"); return (amount: number) => { order.push("make"); return (value: number) => { order.push("apply"); return value + amount; }; }; },
};
const okay = (name: string, value: number): N => { order.push(name); return { kind: "Ok", value }; };
const fail = (name: string): N => { order.push(name); return { kind: "Err", error: name }; };
const report = (value: unknown) => { console.log(order.join(","), String(value)); order.length = 0; };
report(mark("head", new Box(1)) |> .add(match (subject("arg")) { A(value) => value, B => 0 }) |> .n);
report(mark("head", new Box(1)) |> .add(match (subject("one")) { A(value) => value, B => 0 }) |> .add(match (subject("two")) { A(value) => value, B => 0 }) |> .n);
report(mark("head", new Box(1)) |> .add(1).add(match (subject("arg")) { A(value) => value, B => 0 }).n);
report(mark("head", { list: [10, 20] }) |> .list[match (subject("index")) { A(value) => value, B => 0 }]);
report(mark("head", new Box(1) as Box | undefined) |> ?.add(match (subject("arg")) { A(value) => value, B => 0 }) |> String);
report(mark("head", undefined as Box | undefined) |> ?.add(match (subject("skipped")) { A(value) => value, B => 0 }) |> String);
report(mark("head", 2) |> factory.make(match (subject("arg")) { A(value) => value, B => 0 }));
function lifted(ok: boolean): R {
  const value = mark("head", new Box(1)) |> .add(try (ok ? okay("try", 4) : fail("err")));
  return { kind: "Ok", value };
}
const first = lifted(true);
report(first.kind === "Ok" ? first.value.n : first.error);
const second = lifted(false);
report(second.kind === "Ok" ? second.value.n : second.error);

export {};


//// [aHoistedValueInAMemberStepRunsAfterThePipedValueAndItsMethod.ts]
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

type E =
  | { kind: "A"; value: number }
  | { kind: "B" };
const E = {
  A: (value: number): E => ({ kind: "A", value }),
  B: { kind: "B" } as const,
};
type N = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
type R = { kind: "Ok"; value: Box } | { kind: "Err"; error: string };
const order: string[] = [];
const mark = <T,>(name: string, value: T): T => { order.push(name); return value; };
const subject = (name: string): E => { order.push(name); return E.A(1); };
class Box {
  constructor(readonly n: number) {}
  get add() { order.push("get"); return Box.prototype.addTo; }
  addTo(amount: number): Box { order.push("call"); return new Box(this.n + amount); }
}
const factory = {
  get make() { order.push("get"); return (amount: number) => { order.push("make"); return (value: number) => { order.push("apply"); return value + amount; }; }; },
};
const okay = (name: string, value: number): N => { order.push(name); return { kind: "Ok", value }; };
const fail = (name: string): N => { order.push(name); return { kind: "Err", error: name }; };
const report = (value: unknown) => { console.log(order.join(","), String(value)); order.length = 0; };
let $tt_v0: number;
const $tt_v2: typeof report = (report);
do {
  const $tt_v40 = mark("head", new Box(1));
  let $tt_v1: number;
  const $tt_v4 = ($tt_v40);
  {
    const $tt_m = subject("arg");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v1 = value;
        break;
      }
      case "B": {
        $tt_v1 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const $tt_v41 = $tt_v4.add($tt_v1);
  $tt_v0 = $tt_v41.n;
  break;
} while (false);
$tt_v2($tt_v0);
let $tt_v5: number;
const $tt_v8: typeof report = (report);
do {
  const $tt_v42 = mark("head", new Box(1));
  let $tt_v6: number;
  const $tt_v10 = ($tt_v42);
  {
    const $tt_m = subject("one");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v6 = value;
        break;
      }
      case "B": {
        $tt_v6 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const $tt_v43 = $tt_v10.add($tt_v6);
  let $tt_v7: number;
  const $tt_v12 = ($tt_v43);
  {
    const $tt_m = subject("two");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v7 = value;
        break;
      }
      case "B": {
        $tt_v7 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const $tt_v44 = $tt_v12.add($tt_v7);
  $tt_v5 = $tt_v44.n;
  break;
} while (false);
$tt_v8($tt_v5);
let $tt_v13: number;
const $tt_v15: typeof report = (report);
do {
  const $tt_v45 = mark("head", new Box(1));
  let $tt_v14: number;
  const $tt_v17 = ($tt_v45.add(1));
  {
    const $tt_m = subject("arg");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v14 = value;
        break;
      }
      case "B": {
        $tt_v14 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v13 = $tt_v17.add($tt_v14).n;
  break;
} while (false);
$tt_v15($tt_v13);
let $tt_v18: number;
const $tt_v20: typeof report = (report);
do {
  const $tt_v46 = mark("head", { list: [10, 20] });
  let $tt_v19: number;
  const $tt_v21 = ($tt_v46.list);
  {
    const $tt_m = subject("index");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v19 = value;
        break;
      }
      case "B": {
        $tt_v19 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v18 = $tt_v21[$tt_v19];
  break;
} while (false);
$tt_v20($tt_v18);
let $tt_v22: string;
const $tt_v24: typeof report = (report);
do {
  const $tt_v47 = mark("head", new Box(1) as Box | undefined);
  let $tt_v27: (Box) | (undefined);
  const $tt_v26 = ($tt_v47);
  if ($tt_v26 != null) {
    {
      const $tt_m = subject("arg");
      switch ($tt_m.kind) {
        case "A": {
          const { value } = $tt_m;
          $tt_v27 = $tt_v26?.add(value);
          break;
        }
        case "B": {
          $tt_v27 = $tt_v26?.add(0);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v27 = undefined;
  }
  const $tt_v48 = $tt_v27;
  $tt_v22 = String($tt_v48);
  break;
} while (false);
$tt_v24($tt_v22);
let $tt_v28: string;
const $tt_v30: typeof report = (report);
do {
  const $tt_v49 = mark("head", undefined as Box | undefined);
  let $tt_v33: (Box) | (undefined);
  const $tt_v32 = ($tt_v49);
  if ($tt_v32 != null) {
    {
      const $tt_m = subject("skipped");
      switch ($tt_m.kind) {
        case "A": {
          const { value } = $tt_m;
          $tt_v33 = $tt_v32?.add(value);
          break;
        }
        case "B": {
          $tt_v33 = $tt_v32?.add(0);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v33 = undefined;
  }
  const $tt_v50 = $tt_v33;
  $tt_v28 = String($tt_v50);
  break;
} while (false);
$tt_v30($tt_v28);
let $tt_v34: number;
const $tt_v36: typeof report = (report);
do {
  const $tt_v51 = mark("head", 2);
  let $tt_v35: number;
  {
    const $tt_m = subject("arg");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v35 = value;
        break;
      }
      case "B": {
        $tt_v35 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v34 = factory.make($tt_v35)($tt_v51);
  break;
} while (false);
$tt_v36($tt_v34);
function lifted(ok: boolean): R {
  let $tt_v38: Box;
  do {
    const $tt_v52 = mark("head", new Box(1));
    let $tt_v39: number;
    const $tt_v54 = ($tt_v52);
    const $tt_t0 = (ok ? okay("try", 4) : fail("err"));
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v39 = $tt_t0.value;
    $tt_v38 = $tt_v54.add($tt_v39);
    break;
  } while (false);
  const value = $tt_v38;
  return { kind: "Ok", value };
}
const first = lifted(true);
report(first.kind === "Ok" ? first.value.n : first.error);
const second = lifted(false);
report(second.kind === "Ok" ? second.value.n : second.error);

export {};
