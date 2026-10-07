//// [runtimeAPropagatedCallAroundAMatchDoesNotShareTheMatchSSlot.tt] ////

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function r(n: number): R { return n > 1 ? { kind: "Ok", value: n } : { kind: "Err", error: "small" + n }; }
function sum(v: number): R {
  const y = 1 + try r(match (v) { 1 => 1, _ => 2 });
  return r(y - 1 + 10);
}
function listed(v: number): R {
  const a = [try r(match (v) { 1 => 1, _ => 2 })];
  console.log(try r(match (v) { 1 => 3, _ => 4 }));
  return { kind: "Ok", value: a[0] };
}
function inResult(v: number) {
  return result {
    const y = 1 + try r(match (v) { 1 => 1, _ => 2 });
    const a = [try r(match (v) { 1 => 1, _ => 2 })];
    console.log(try r(match (v) { 1 => 3, _ => 4 }));
    return y + a[0];
  };
}
console.log(JSON.stringify([sum(1), sum(2), listed(1), listed(2), inResult(1), inResult(2)]));

export {};


//// [runtimeAPropagatedCallAroundAMatchDoesNotShareTheMatchSSlot.ts]

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function r(n: number): R { return n > 1 ? { kind: "Ok", value: n } : { kind: "Err", error: "small" + n }; }
function sum(v: number): R {
  let $tt_v0: number;
  let $tt_v1: number;
  const $tt_v2: typeof r = (r);
  {
    const $tt_m = v;
    switch ($tt_m) {
      case 1: {
        $tt_v1 = 1;
        break;
      }
      default: {
        $tt_v1 = 2;
        break;
      }
    }
  }
  const $tt_t0 = $tt_v2($tt_v1);
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  const y = 1 + $tt_v0;
  return r(y - 1 + 10);
}
function listed(v: number): R {
  let $tt_v3: number;
  let $tt_v4: number;
  const $tt_v5: typeof r = (r);
  {
    const $tt_m = v;
    switch ($tt_m) {
      case 1: {
        $tt_v4 = 1;
        break;
      }
      default: {
        $tt_v4 = 2;
        break;
      }
    }
  }
  const $tt_t1 = $tt_v5($tt_v4);
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v3 = $tt_t1.value;
  const a = [$tt_v3];
  let $tt_v6: number;
  let $tt_v7: number;
  const $tt_v9: typeof r = (r);
  {
    const $tt_m = v;
    switch ($tt_m) {
      case 1: {
        $tt_v7 = 3;
        break;
      }
      default: {
        $tt_v7 = 4;
        break;
      }
    }
  }
  const $tt_t2 = $tt_v9($tt_v7);
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  $tt_v6 = $tt_t2.value;
  console.log($tt_v6);
  return { kind: "Ok", value: a[0] };
}
function inResult(v: number) {
  let $tt_v10: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v10: {
    let $tt_v11: number;
    let $tt_v12: number;
    const $tt_v13: typeof r = (r);
    {
      const $tt_m = v;
      switch ($tt_m) {
        case 1: {
          $tt_v12 = 1;
          break;
        }
        default: {
          $tt_v12 = 2;
          break;
        }
      }
    }
    const $tt_t3 = $tt_v13($tt_v12);
    if (!("value" in $tt_t3)) {
      $tt_v10 = $tt_t3;
      break $tt_v10;
    }
    $tt_v11 = $tt_t3.value;
    const y = 1 + $tt_v11;
    let $tt_v14: number;
    let $tt_v15: number;
    const $tt_v16: typeof r = (r);
    {
      const $tt_m = v;
      switch ($tt_m) {
        case 1: {
          $tt_v15 = 1;
          break;
        }
        default: {
          $tt_v15 = 2;
          break;
        }
      }
    }
    const $tt_t4 = $tt_v16($tt_v15);
    if (!("value" in $tt_t4)) {
      $tt_v10 = $tt_t4;
      break $tt_v10;
    }
    $tt_v14 = $tt_t4.value;
    const a = [$tt_v14];
    let $tt_v17: number;
    let $tt_v18: number;
    const $tt_v20: typeof r = (r);
    {
      const $tt_m = v;
      switch ($tt_m) {
        case 1: {
          $tt_v18 = 3;
          break;
        }
        default: {
          $tt_v18 = 4;
          break;
        }
      }
    }
    const $tt_t5 = $tt_v20($tt_v18);
    if (!("value" in $tt_t5)) {
      $tt_v10 = $tt_t5;
      break $tt_v10;
    }
    $tt_v17 = $tt_t5.value;
    console.log($tt_v17);
    {
      $tt_v10 = { kind: "Ok" as const, value: y + a[0] };
      break $tt_v10;
    }
  }
  return $tt_v10;
}
console.log(JSON.stringify([sum(1), sum(2), listed(1), listed(2), inResult(1), inResult(2)]));

export {};
