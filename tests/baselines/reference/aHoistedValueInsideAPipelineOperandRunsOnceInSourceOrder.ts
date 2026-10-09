//// [aHoistedValueInsideAPipelineOperandRunsOnceInSourceOrder.tt] ////

variant E { A(value: number), B }
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const order: string[] = [];
const mark = <T,>(name: string, value: T): T => { order.push(name); return value; };
const subject = (name: string): E => { order.push(name); return E.A(1); };
const add = (value: number) => { order.push("call"); return value + 1; };
const callee = () => { order.push("callee"); return add; };
const make = (n: number) => { order.push("make"); return (value: number) => { order.push("apply"); return value + n; }; };
const step = () => { order.push("step"); return (value: number) => { order.push("apply"); return value * 10; }; };
const okay = (name: string, value: number): R => { order.push(name); return { kind: "Ok", value }; };
const fail = (name: string): R => { order.push(name); return { kind: "Err", error: name }; };
const report = (value: unknown) => { console.log(order.join(","), String(value)); order.length = 0; };
report(callee()(match (subject("head")) { A(value) => value, B => 0 }) |> step());
report(mark("left", 1) + match (subject("right")) { A(value) => value, B => 0 } |> step());
report(match (subject("head")) { A(value) => value, B => 0 } + mark("right", 1) |> step());
report([mark("first", 1), match (subject("second")) { A(value) => value, B => 0 }] |> (pair => pair.map(n => n + 1)));
report(-match (subject("head")) { A(value) => value, B => 0 } |> step());
report((match (subject("head")) { A(value) => value, B => 0 }).toFixed(1) |> Number);
report(`${callee()(match (subject("head")) { A(value) => value, B => 0 })}` |> Number);
report(mark("head", 3) |> make(match (subject("arg")) { A(value) => value, B => 0 }));
report(callee()(match (subject("one")) { A(value) => value, B => 0 }) + callee()(match (subject("two")) { A(value) => value, B => 0 }) |> step());
report(mark("cond", true) && callee()(match (subject("branch")) { A(value) => value, B => 0 }) |> String);
report(mark("cond", false) && callee()(match (subject("skipped")) { A(value) => value, B => 0 }) |> String);
function lifted(ok: boolean): R {
  const value = callee()(try (ok ? okay("try", 4) : fail("err"))) |> step();
  return { kind: "Ok", value };
}
const first = lifted(true);
report(first.kind === "Ok" ? first.value : first.error);
const second = lifted(false);
report(second.kind === "Ok" ? second.value : second.error);

export {};


//// [aHoistedValueInsideAPipelineOperandRunsOnceInSourceOrder.ts]
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
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const order: string[] = [];
const mark = <T,>(name: string, value: T): T => { order.push(name); return value; };
const subject = (name: string): E => { order.push(name); return E.A(1); };
const add = (value: number) => { order.push("call"); return value + 1; };
const callee = () => { order.push("callee"); return add; };
const make = (n: number) => { order.push("make"); return (value: number) => { order.push("apply"); return value + n; }; };
const step = () => { order.push("step"); return (value: number) => { order.push("apply"); return value * 10; }; };
const okay = (name: string, value: number): R => { order.push(name); return { kind: "Ok", value }; };
const fail = (name: string): R => { order.push(name); return { kind: "Err", error: name }; };
const report = (value: unknown) => { console.log(order.join(","), String(value)); order.length = 0; };
let $tt_v0: number;
const $tt_v2: typeof report = (report);
do {
  let $tt_v50: number;
  let $tt_v1: number;
  const $tt_v3 = (callee());
  {
    const $tt_m = subject("head");
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
  $tt_v50 = $tt_v3($tt_v1);
  $tt_v0 = step()($tt_v50);
  break;
} while (false);
$tt_v2($tt_v0);
let $tt_v4: number;
const $tt_v6: typeof report = (report);
do {
  let $tt_v51: number;
  let $tt_v5: number;
  const $tt_v7 = (mark("left", 1));
  {
    const $tt_m = subject("right");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v5 = value;
        break;
      }
      case "B": {
        $tt_v5 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v51 = $tt_v7 + $tt_v5;
  $tt_v4 = step()($tt_v51);
  break;
} while (false);
$tt_v6($tt_v4);
let $tt_v8: number;
const $tt_v10: typeof report = (report);
do {
  let $tt_v52: number;
  let $tt_v9: number;
  {
    const $tt_m = subject("head");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v9 = value;
        break;
      }
      case "B": {
        $tt_v9 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v52 = $tt_v9 + mark("right", 1);
  $tt_v8 = step()($tt_v52);
  break;
} while (false);
$tt_v10($tt_v8);
let $tt_v11: number[];
const $tt_v13: typeof report = (report);
do {
  let $tt_v53: number[];
  let $tt_v12: number;
  const $tt_v14 = (mark("first", 1));
  {
    const $tt_m = subject("second");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v12 = value;
        break;
      }
      case "B": {
        $tt_v12 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v53 = [$tt_v14, $tt_v12];
  $tt_v11 = (pair => pair.map(n => n + 1))($tt_v53);
  break;
} while (false);
$tt_v13($tt_v11);
let $tt_v15: number;
const $tt_v17: typeof report = (report);
do {
  let $tt_v54: number;
  let $tt_v16: number;
  {
    const $tt_m = subject("head");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v16 = value;
        break;
      }
      case "B": {
        $tt_v16 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v54 = -$tt_v16;
  $tt_v15 = step()($tt_v54);
  break;
} while (false);
$tt_v17($tt_v15);
let $tt_v18: number;
const $tt_v20: typeof report = (report);
do {
  let $tt_v55: string;
  let $tt_v19: number;
  {
    const $tt_m = subject("head");
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
  $tt_v55 = ($tt_v19).toFixed(1);
  $tt_v18 = Number($tt_v55);
  break;
} while (false);
$tt_v20($tt_v18);
let $tt_v21: number;
const $tt_v23: typeof report = (report);
do {
  let $tt_v22: number;
  const $tt_v24 = (callee());
  {
    const $tt_m = subject("head");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v22 = value;
        break;
      }
      case "B": {
        $tt_v22 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const $tt_v56 = `${$tt_v24($tt_v22)}`;
  $tt_v21 = Number($tt_v56);
  break;
} while (false);
$tt_v23($tt_v21);
let $tt_v25: number;
const $tt_v27: typeof report = (report);
do {
  const $tt_v57 = mark("head", 3);
  let $tt_v26: number;
  const $tt_v28: typeof make = (make);
  {
    const $tt_m = subject("arg");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v26 = value;
        break;
      }
      case "B": {
        $tt_v26 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v25 = $tt_v28($tt_v26)($tt_v57);
  break;
} while (false);
$tt_v27($tt_v25);
let $tt_v29: number;
const $tt_v32: typeof report = (report);
do {
  let $tt_v58: number;
  let $tt_v30: number;
  const $tt_v33 = (callee());
  {
    const $tt_m = subject("one");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v30 = value;
        break;
      }
      case "B": {
        $tt_v30 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  let $tt_v31: number;
  const $tt_v35 = ($tt_v33($tt_v30));
  const $tt_v34 = (callee());
  {
    const $tt_m = subject("two");
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v31 = value;
        break;
      }
      case "B": {
        $tt_v31 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v58 = $tt_v35 + $tt_v34($tt_v31);
  $tt_v29 = step()($tt_v58);
  break;
} while (false);
$tt_v32($tt_v29);
let $tt_v36: string;
const $tt_v38: typeof report = (report);
do {
  let $tt_v59: number;
  let $tt_v41: number;
  let $tt_v40: true;
  if ($tt_v40 = mark("cond", true)) {
    let $tt_v37: number;
    const $tt_v39 = (callee());
    {
      const $tt_m = subject("branch");
      switch ($tt_m.kind) {
        case "A": {
          const { value } = $tt_m;
          $tt_v37 = value;
          break;
        }
        case "B": {
          $tt_v37 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v41 = $tt_v40 && $tt_v39($tt_v37);
  } else {
    $tt_v41 = $tt_v40;
  }
  $tt_v59 = $tt_v41;
  $tt_v36 = String($tt_v59);
  break;
} while (false);
$tt_v38($tt_v36);
let $tt_v42: string;
const $tt_v44: typeof report = (report);
do {
  let $tt_v60: false;
  let $tt_v47: false;
  let $tt_v46: false;
  if ($tt_v46 = mark("cond", false)) {
    let $tt_v43: number;
    const $tt_v45 = (callee());
    {
      const $tt_m = subject("skipped");
      switch ($tt_m.kind) {
        case "A": {
          const { value } = $tt_m;
          $tt_v43 = value;
          break;
        }
        case "B": {
          $tt_v43 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v47 = $tt_v46 && $tt_v45($tt_v43);
  } else {
    $tt_v47 = $tt_v46;
  }
  $tt_v60 = $tt_v47;
  $tt_v42 = String($tt_v60);
  break;
} while (false);
$tt_v44($tt_v42);
function lifted(ok: boolean): R {
  let $tt_v48: number;
  do {
    let $tt_v61: number;
    let $tt_v49: number;
    const $tt_v62 = (callee());
    const $tt_t0 = (ok ? okay("try", 4) : fail("err"));
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v49 = $tt_t0.value;
    $tt_v61 = $tt_v62($tt_v49);
    $tt_v48 = step()($tt_v61);
    break;
  } while (false);
  const value = $tt_v48;
  return { kind: "Ok", value };
}
const first = lifted(true);
report(first.kind === "Ok" ? first.value : first.error);
const second = lifted(false);
report(second.kind === "Ok" ? second.value : second.error);

export {};
