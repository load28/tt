//// [aPropagatedValueRegionKeepsItsBlockReturns.tt] ////

type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const Ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const Err = (error: string): R<never> => ({ kind: "Err", error });
function statement(): R<number> {
    const q = try result { const w = try Ok(10); return w + 1; };
    return Ok(q * 2);
}
function product(fail: boolean): R<number> {
    const q = (try result { const w = try (fail ? Err("no") : Ok(10)); return w + 1; }) * 2;
    return Ok(q);
}
function discarded(): R<number> {
    try result { const w = try Err("inner"); return w; };
    return Ok(0);
}
function matched(b: boolean): R<number> {
    const q = (try match (b) { true => { return Ok(10); }, false => Ok(1) }) * 2;
    return Ok(q + 1000);
}
function guarded(ready: boolean, b: boolean): R<number> {
    const q = ready && (try match (b) { true => { return Ok(10); }, false => Ok(1) });
    return Ok(Number(q) + 1000);
}
function argument(b: boolean): R<number> {
    const q = String(try match (b) { true => { return Ok(10); }, false => Ok(1) });
    return Ok(Number(q) + 1000);
}
function alternate(ready: boolean): R<number> {
    const q = ready ? 5 : (try result { const w = try Ok(3); return w + 1; });
    return Ok(q);
}
function piped(): R<string> {
    const q = (try result { const w = try Ok(3); return w + 1; }) |> String;
    return Ok(q);
}
function interpolated(b: boolean): R<number> {
    const q = `${try match (b) { true => { return Ok(7); }, false => Err("x") }}`;
    return Ok(Number(q) + 1);
}
const nested = result { const q = try result { const w = try Ok(3); return w + 1; }; return q * 2; };
console.log(JSON.stringify([statement(), product(false), product(true), discarded()]));
console.log(JSON.stringify([matched(true), guarded(true, true), guarded(false, true), argument(true)]));
console.log(JSON.stringify([alternate(false), piped(), interpolated(true), nested]));

export {};


//// [aPropagatedValueRegionKeepsItsBlockReturns.ts]
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

type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const Ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const Err = (error: string): R<never> => ({ kind: "Err", error });
function statement(): R<number> {
    let $tt_v16: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
    $tt_v16: {
      const $tt_t1 = Ok(10);
      if (!("value" in $tt_t1)) {
        $tt_v16 = $tt_t1;
        break $tt_v16;
      }
      const w = $tt_t1.value; { const $tt_a0 = { value: { kind: "Ok" as const, value: w + 1 } }; $tt_v16 = $tt_a0.value; break $tt_v16; }
    }
    const $tt_t0 = $tt_v16;
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    const q = $tt_t0.value;
    return Ok(q * 2);
}
function product(fail: boolean): R<number> {
    let $tt_v0: number;
    let $tt_v17: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
    $tt_v17: {
      const $tt_t3 = (fail ? Err("no") : Ok(10));
      if (!("value" in $tt_t3)) {
        $tt_v17 = $tt_t3;
        break $tt_v17;
      }
      const w = $tt_t3.value; { const $tt_a1 = { value: { kind: "Ok" as const, value: w + 1 } }; $tt_v17 = $tt_a1.value; break $tt_v17; }
    }
    const $tt_t2 = $tt_v17;
    if (!("value" in $tt_t2)) {
      return $tt_t2;
    }
    $tt_v0 = $tt_t2.value;
    const q = ($tt_v0) * 2;
    return Ok(q);
}
function discarded(): R<number> {
    let $tt_v18: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: never;
});
    $tt_v18: {
      const $tt_t5 = Err("inner");
      if (!("value" in $tt_t5)) {
        $tt_v18 = $tt_t5;
        break $tt_v18;
      }
      const w = $tt_t5.value; { const $tt_a2 = { value: { kind: "Ok" as const, value: w } }; $tt_v18 = $tt_a2.value; break $tt_v18; }
    }
    const $tt_t4 = $tt_v18;
    if (!("value" in $tt_t4)) {
      return $tt_t4;
    }
    return Ok(0);
}
function matched(b: boolean): R<number> {
    let $tt_v1: number;
    let $tt_v2: R<number>;
    {
      const $tt_m = b;
      switch ($tt_m) {
        case true: {
          $tt_v2 = Ok(10); break;
        }
        case false: {
          $tt_v2 = Ok(1);
          break;
        }
        default: {
          throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
        }
      }
    }
    const $tt_t6 = $tt_v2;
    if (!("value" in $tt_t6)) {
      return $tt_t6;
    }
    $tt_v1 = $tt_t6.value;
    const q = ($tt_v1) * 2;
    return Ok(q + 1000);
}
function guarded(ready: boolean, b: boolean): R<number> {
    let $tt_v6: (number) | (false);
    let $tt_v5: boolean;
    if ($tt_v5 = ready) {
      let $tt_v3: number;
      let $tt_v4: R<number>;
      {
        const $tt_m = b;
        switch ($tt_m) {
          case true: {
            $tt_v4 = Ok(10); break;
          }
          case false: {
            $tt_v4 = Ok(1);
            break;
          }
          default: {
            throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
          }
        }
      }
      const $tt_t7 = $tt_v4;
      if (!("value" in $tt_t7)) {
        return $tt_t7;
      }
      $tt_v3 = $tt_t7.value;
      $tt_v6 = $tt_v5 && $tt_v3;
    } else {
      $tt_v6 = $tt_v5;
    }
    
    const q = $tt_v6;
    return Ok(Number(q) + 1000);
}
function argument(b: boolean): R<number> {
    let $tt_v7: number;
    const $tt_v9 = (String);
    let $tt_v8: R<number>;
    {
      const $tt_m = b;
      switch ($tt_m) {
        case true: {
          $tt_v8 = Ok(10); break;
        }
        case false: {
          $tt_v8 = Ok(1);
          break;
        }
        default: {
          throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
        }
      }
    }
    const $tt_t8 = $tt_v8;
    if (!("value" in $tt_t8)) {
      return $tt_t8;
    }
    $tt_v7 = $tt_t8.value;
    const q = $tt_v9($tt_v7);
    return Ok(Number(q) + 1000);
}
function alternate(ready: boolean): R<number> {
    let $tt_v12: number;
    if (ready) {
      $tt_v12 = 5;
    } else {
      let $tt_v19: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
      $tt_v19: {
        const $tt_t10 = Ok(3);
        if (!("value" in $tt_t10)) {
          $tt_v19 = $tt_t10;
          break $tt_v19;
        }
        const w = $tt_t10.value; { const $tt_a3 = { value: { kind: "Ok" as const, value: w + 1 } }; $tt_v19 = $tt_a3.value; break $tt_v19; }
      }
      const $tt_t9 = $tt_v19;
      if (!("value" in $tt_t9)) {
        return $tt_t9;
      }
      $tt_v12 = $tt_t9.value;
    }
    
    const q = $tt_v12;
    return Ok(q);
}
function piped(): R<string> {
    let $tt_v13: string;
    do {
      let $tt_v24: number;
      let $tt_v20: number;
      let $tt_v21: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
      $tt_v21: {
        const $tt_t12 = Ok(3);
        if (!("value" in $tt_t12)) {
          $tt_v21 = $tt_t12;
          break $tt_v21;
        }
        const w = $tt_t12.value; { const $tt_a4 = { value: { kind: "Ok" as const, value: w + 1 } }; $tt_v21 = $tt_a4.value; break $tt_v21; }
      }
      const $tt_t11 = $tt_v21;
      if (!("value" in $tt_t11)) {
        return $tt_t11;
      }
      $tt_v20 = $tt_t11.value;
      $tt_v24 = ($tt_v20);
      $tt_v13 = String($tt_v24);
      break;
    } while (false);
    const q = $tt_v13;
    return Ok(q);
}
function interpolated(b: boolean): R<number> {
    let $tt_v14: number;
    let $tt_v22: R<number>;
    {
      const $tt_m = b;
      switch ($tt_m) {
        case true: {
          $tt_v22 = Ok(7); break;
        }
        case false: {
          $tt_v22 = Err("x");
          break;
        }
        default: {
          throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
        }
      }
    }
    const $tt_t13 = $tt_v22;
    if (!("value" in $tt_t13)) {
      return $tt_t13;
    }
    $tt_v14 = $tt_t13.value;
    const q = `${$tt_v14}`;
    return Ok(Number(q) + 1);
}
let $tt_v15: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v15: {
  let $tt_v23: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v23: {
    const $tt_t15 = Ok(3);
    if (!("value" in $tt_t15)) {
      $tt_v23 = $tt_t15;
      break $tt_v23;
    }
    const w = $tt_t15.value; { const $tt_a5 = { value: { kind: "Ok" as const, value: w + 1 } }; $tt_v23 = $tt_a5.value; break $tt_v23; }
  }
  const $tt_t14 = $tt_v23;
  if (!("value" in $tt_t14)) {
    $tt_v15 = $tt_t14;
    break $tt_v15;
  }
  const q = $tt_t14.value; { const $tt_a6 = { value: { kind: "Ok" as const, value: q * 2 } }; $tt_v15 = $tt_a6.value; break $tt_v15; }
}
const nested = $tt_v15;
console.log(JSON.stringify([statement(), product(false), product(true), discarded()]));
console.log(JSON.stringify([matched(true), guarded(true, true), guarded(false, true), argument(true)]));
console.log(JSON.stringify([alternate(false), piped(), interpolated(true), nested]));

export {};
