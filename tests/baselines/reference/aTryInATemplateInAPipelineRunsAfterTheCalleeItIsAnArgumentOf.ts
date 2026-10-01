//// [aTryInATemplateInAPipelineRunsAfterTheCalleeItIsAnArgumentOf.tt] ////

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
type S = { kind: "Ok"; value: string } | { kind: "Err"; error: string };
const order: string[] = [];
const wrap = (value: number) => { order.push("call"); return `<${value}>`; };
const callee = () => { order.push("callee"); return wrap; };
const suffix = (tail: string) => { order.push("step"); return (value: string) => { order.push("apply"); return value + tail; }; };
const okay = (name: string, value: number): R => { order.push(name); return { kind: "Ok", value }; };
const fail = (name: string): R => { order.push(name); return { kind: "Err", error: name }; };
const report = (value: S) => { console.log(order.join(","), value.kind === "Ok" ? value.value : value.error); order.length = 0; };
function head(ok: boolean): S {
  const value = `${callee()(try (ok ? okay("try", 1) : fail("err")))}!` |> String;
  return { kind: "Ok", value };
}
function step(ok: boolean): S {
  const value = "v" |> suffix(`${callee()(try (ok ? okay("try", 2) : fail("err")))}`);
  return { kind: "Ok", value };
}
function nested(ok: boolean): S {
  const value = callee()(`${callee()(try (ok ? okay("try", 3) : fail("err")))}`.length) |> String;
  return { kind: "Ok", value };
}
report(head(true));
report(head(false));
report(step(true));
report(step(false));
report(nested(true));
report(nested(false));

export {};


//// [aTryInATemplateInAPipelineRunsAfterTheCalleeItIsAnArgumentOf.ts]

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
type S = { kind: "Ok"; value: string } | { kind: "Err"; error: string };
const order: string[] = [];
const wrap = (value: number) => { order.push("call"); return `<${value}>`; };
const callee = () => { order.push("callee"); return wrap; };
const suffix = (tail: string) => { order.push("step"); return (value: string) => { order.push("apply"); return value + tail; }; };
const okay = (name: string, value: number): R => { order.push(name); return { kind: "Ok", value }; };
const fail = (name: string): R => { order.push(name); return { kind: "Err", error: name }; };
const report = (value: S) => { console.log(order.join(","), value.kind === "Ok" ? value.value : value.error); order.length = 0; };
function head(ok: boolean): S {
  let $tt_v0: string;
  do {
    let $tt_v3: number;
    const $tt_v9 = (callee());
    const $tt_t0 = (ok ? okay("try", 1) : fail("err"));
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v3 = $tt_t0.value;
    const $tt_v6 = `${$tt_v9($tt_v3)}!`;
    $tt_v0 = String($tt_v6);
    break;
  } while (false);
  const value = $tt_v0;
  return { kind: "Ok", value };
}
function step(ok: boolean): S {
  let $tt_v1: string;
  do {
    const $tt_v7 = "v";
    let $tt_v4: number;
    const $tt_v11 = (suffix);
    const $tt_v10 = (callee());
    const $tt_t1 = (ok ? okay("try", 2) : fail("err"));
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    $tt_v4 = $tt_t1.value;
    $tt_v1 = $tt_v11(`${$tt_v10($tt_v4)}`)($tt_v7);
    break;
  } while (false);
  const value = $tt_v1;
  return { kind: "Ok", value };
}
function nested(ok: boolean): S {
  let $tt_v2: string;
  do {
    let $tt_v5: number;
    const $tt_v13 = (callee());
    const $tt_v12 = (callee());
    const $tt_t2 = (ok ? okay("try", 3) : fail("err"));
    if (!("value" in $tt_t2)) {
      return $tt_t2;
    }
    $tt_v5 = $tt_t2.value;
    const $tt_v8 = $tt_v13(`${$tt_v12($tt_v5)}`.length);
    $tt_v2 = String($tt_v8);
    break;
  } while (false);
  const value = $tt_v2;
  return { kind: "Ok", value };
}
report(head(true));
report(head(false));
report(step(true));
report(step(false));
report(nested(true));
report(nested(false));

export {};
