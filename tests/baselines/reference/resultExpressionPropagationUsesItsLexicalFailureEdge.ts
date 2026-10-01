//// [resultExpressionPropagationUsesItsLexicalFailureEdge.tt] ////

type R<T> = {kind: "Ok"; value: T} | {kind: "Err"; error: string};
const trace: string[] = [];
function read(ok: boolean): R<number> { trace.push("read"); return ok ? {kind: "Ok", value: 3} : {kind: "Err", error: "bad"}; }
function before() { trace.push("before"); return 2; }
function compute(ok: boolean) {
  const r = result {
    let company;
    company = before() + try read(ok);
    trace.push("after");
    return company;
  };
  trace.push("outside");
  return r;
}
for (const ok of [true, false]) {
  trace.length = 0;
  console.log(JSON.stringify(compute(ok)), trace.join(","));
}
const nested = result {
  const a = try read(true);
  const inner = result { let b; b = try read(false); return b; };
  return [a, inner.kind];
};
console.log(JSON.stringify(nested));
const skipped = result { let n = 0; n = false ? try read(false) : 4; return n; };
console.log(JSON.stringify(skipped));

export {};


//// [resultExpressionPropagationUsesItsLexicalFailureEdge.ts]

type R<T> = {kind: "Ok"; value: T} | {kind: "Err"; error: string};
const trace: string[] = [];
function read(ok: boolean): R<number> { trace.push("read"); return ok ? {kind: "Ok", value: 3} : {kind: "Err", error: "bad"}; }
function before() { trace.push("before"); return 2; }
function compute(ok: boolean) {
  let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v0: {
    let company;
    let $tt_v1: number;
    const $tt_v2 = (before());
    const $tt_t0 = read(ok);
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    $tt_v1 = $tt_t0.value;
    company = $tt_v2 + $tt_v1;
    trace.push("after");
    {
      const $tt_a0 = { value: { kind: "Ok" as const, value: company } };
      $tt_v0 = $tt_a0.value;
      break $tt_v0;
    }
  }
  const r = $tt_v0;
  trace.push("outside");
  return r;
}
for (const ok of [true, false]) {
  trace.length = 0;
  console.log(JSON.stringify(compute(ok)), trace.join(","));
}
let $tt_v3: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: (number | "Err" | "Ok")[];
});
$tt_v3: {
  const $tt_t1 = read(true);
  if (!("value" in $tt_t1)) {
    $tt_v3 = $tt_t1;
    break $tt_v3;
  }
  const a = $tt_t1.value;
  let $tt_v4: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v4: {
    let b; let $tt_v5: number;
    const $tt_t2 = read(false);
    if (!("value" in $tt_t2)) {
      $tt_v4 = $tt_t2;
      break $tt_v4;
    }
    $tt_v5 = $tt_t2.value;
    b = $tt_v5; { const $tt_a1 = { value: { kind: "Ok" as const, value: b } }; $tt_v4 = $tt_a1.value; break $tt_v4; }
  }
  const inner = $tt_v4;
  {
    const $tt_a2 = { value: { kind: "Ok" as const, value: [a, inner.kind] } };
    $tt_v3 = $tt_a2.value;
    break $tt_v3;
  }
}
const nested = $tt_v3;
console.log(JSON.stringify(nested));
let $tt_v6: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v6: {
  let n = 0; let $tt_v8: number;
  if ((false)) {
    const $tt_t3 = read(false);
    if (!("value" in $tt_t3)) {
      $tt_v6 = $tt_t3;
      break $tt_v6;
    }
    $tt_v8 = $tt_t3.value;
  } else {
    $tt_v8 = 4;
  }
  
  n = $tt_v8; { const $tt_a3 = { value: { kind: "Ok" as const, value: n } }; $tt_v6 = $tt_a3.value; break $tt_v6; }
}
const skipped = $tt_v6;
console.log(JSON.stringify(skipped));

export {};
