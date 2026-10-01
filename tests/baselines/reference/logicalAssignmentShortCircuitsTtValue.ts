//// [main.tt] ////
export {};
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const log: string[] = [];
function read(n: number): R {
  log.push("read");
  return n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "neg" };
}
function nullish(o: { a?: number }, k: number): R {
  o.a ??= try read(k);
  return { kind: "Ok", value: o.a };
}
const counted = {
  stored: undefined as number | undefined,
  get a(): number | undefined {
    log.push("get a");
    return this.stored;
  },
  set a(value: number | undefined) {
    log.push("set a");
    this.stored = value;
  },
};
function pick<T>(value: T): T {
  log.push("object");
  return value;
}
function key(): string {
  log.push("key");
  return "a";
}
function getterOnce(k: number): R {
  counted.a ??= try read(k);
  return { kind: "Ok", value: counted.a };
}
function computedOnce(o: Record<string, number | undefined>, k: number): R {
  pick(o)[key()] ??= try read(k);
  return { kind: "Ok", value: o.a ?? -1 };
}
function or(o: { b: boolean }): boolean {
  o.b ||= match (log.push("scrutinee") > 0) { true => true, false => false };
  return o.b;
}
function and(o: { c: number }, k: number): R {
  o.c &&= try read(k);
  return { kind: "Ok", value: o.c };
}
console.log(JSON.stringify(nullish({ a: 1 }, -1)), log.splice(0).join());
console.log(JSON.stringify(nullish({}, 2)), log.splice(0).join());
console.log(or({ b: true }), log.splice(0).join());
console.log(or({ b: false }), log.splice(0).join());
console.log(JSON.stringify(and({ c: 0 }, -1)), log.splice(0).join());
console.log(JSON.stringify(and({ c: 1 }, -1)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(-1)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(4)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(-1)), log.splice(0).join());
console.log(JSON.stringify(computedOnce({}, 5)), log.splice(0).join());
console.log(JSON.stringify(computedOnce({ a: 2 }, -5)), log.splice(0).join());

//// [twin.ts] ////
export {};
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const log: string[] = [];
function read(n: number): R {
  log.push("read");
  return n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "neg" };
}
function nullish(o: { a?: number }, k: number): R {
  if (o.a == null) {
    const r = read(k);
    if (!("value" in r)) return r;
    o.a = r.value;
  }
  return { kind: "Ok", value: o.a };
}
const counted = {
  stored: undefined as number | undefined,
  get a(): number | undefined {
    log.push("get a");
    return this.stored;
  },
  set a(value: number | undefined) {
    log.push("set a");
    this.stored = value;
  },
};
function pick<T>(value: T): T {
  log.push("object");
  return value;
}
function key(): string {
  log.push("key");
  return "a";
}
function getterOnce(k: number): R {
  if (counted.a == null) {
    const r = read(k);
    if (!("value" in r)) return r;
    counted.a = r.value;
  }
  return { kind: "Ok", value: counted.a };
}
function computedOnce(o: Record<string, number | undefined>, k: number): R {
  const target = pick(o);
  const name = key();
  if (target[name] == null) {
    const r = read(k);
    if (!("value" in r)) return r;
    target[name] = r.value;
  }
  return { kind: "Ok", value: o.a ?? -1 };
}
function or(o: { b: boolean }): boolean {
  o.b ||= log.push("scrutinee") > 0 ? true : false;
  return o.b;
}
function and(o: { c: number }, k: number): R {
  if (o.c) {
    const r = read(k);
    if (!("value" in r)) return r;
    o.c = r.value;
  }
  return { kind: "Ok", value: o.c };
}
console.log(JSON.stringify(nullish({ a: 1 }, -1)), log.splice(0).join());
console.log(JSON.stringify(nullish({}, 2)), log.splice(0).join());
console.log(or({ b: true }), log.splice(0).join());
console.log(or({ b: false }), log.splice(0).join());
console.log(JSON.stringify(and({ c: 0 }, -1)), log.splice(0).join());
console.log(JSON.stringify(and({ c: 1 }, -1)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(-1)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(4)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(-1)), log.splice(0).join());
console.log(JSON.stringify(computedOnce({}, 5)), log.splice(0).join());
console.log(JSON.stringify(computedOnce({ a: 2 }, -5)), log.splice(0).join());


//// [main.ts]
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
export {};
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const log: string[] = [];
function read(n: number): R {
  log.push("read");
  return n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "neg" };
}
function nullish(o: { a?: number }, k: number): R {
  if (o.a == null) {
    let $tt_v0: number | undefined;
    const $tt_t0 = read(k);
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v0 = $tt_t0.value;
    o.a = $tt_v0;
  }
  
  ;
  return { kind: "Ok", value: o.a };
}
const counted = {
  stored: undefined as number | undefined,
  get a(): number | undefined {
    log.push("get a");
    return this.stored;
  },
  set a(value: number | undefined) {
    log.push("set a");
    this.stored = value;
  },
};
function pick<T>(value: T): T {
  log.push("object");
  return value;
}
function key(): string {
  log.push("key");
  return "a";
}
function getterOnce(k: number): R {
  if (counted.a == null) {
    let $tt_v3: number | undefined;
    const $tt_t1 = read(k);
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    $tt_v3 = $tt_t1.value;
    counted.a = $tt_v3;
  }
  
  ;
  return { kind: "Ok", value: counted.a };
}
function computedOnce(o: Record<string, number | undefined>, k: number): R {
  const $tt_v8 = (pick(o));
  const $tt_v9 = (key());
  if ($tt_v8[$tt_v9] == null) {
    let $tt_v6: number | undefined;
    const $tt_t2 = read(k);
    if (!("value" in $tt_t2)) {
      return $tt_t2;
    }
    $tt_v6 = $tt_t2.value;
    $tt_v8[$tt_v9] = $tt_v6;
  }
  
  ;
  return { kind: "Ok", value: o.a ?? -1 };
}
function or(o: { b: boolean }): boolean {
  if (!o.b) {
    let $tt_v11: boolean;
    {
      const $tt_m = log.push("scrutinee") > 0;
      switch ($tt_m) {
        case true: {
          $tt_v11 = true;
          break;
        }
        case false: {
          $tt_v11 = false;
          break;
        }
        default: {
          throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
        }
      }
    }
    o.b = $tt_v11;
  }
  
  ;
  return o.b;
}
function and(o: { c: number }, k: number): R {
  if (o.c) {
    let $tt_v14: number;
    const $tt_t3 = read(k);
    if (!("value" in $tt_t3)) {
      return $tt_t3;
    }
    $tt_v14 = $tt_t3.value;
    o.c = $tt_v14;
  }
  
  ;
  return { kind: "Ok", value: o.c };
}
console.log(JSON.stringify(nullish({ a: 1 }, -1)), log.splice(0).join());
console.log(JSON.stringify(nullish({}, 2)), log.splice(0).join());
console.log(or({ b: true }), log.splice(0).join());
console.log(or({ b: false }), log.splice(0).join());
console.log(JSON.stringify(and({ c: 0 }, -1)), log.splice(0).join());
console.log(JSON.stringify(and({ c: 1 }, -1)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(-1)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(4)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(-1)), log.splice(0).join());
console.log(JSON.stringify(computedOnce({}, 5)), log.splice(0).join());
console.log(JSON.stringify(computedOnce({ a: 2 }, -5)), log.splice(0).join());
\ No newline at end of file

//// [twin.ts]
export {};
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const log: string[] = [];
function read(n: number): R {
  log.push("read");
  return n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "neg" };
}
function nullish(o: { a?: number }, k: number): R {
  if (o.a == null) {
    const r = read(k);
    if (!("value" in r)) return r;
    o.a = r.value;
  }
  return { kind: "Ok", value: o.a };
}
const counted = {
  stored: undefined as number | undefined,
  get a(): number | undefined {
    log.push("get a");
    return this.stored;
  },
  set a(value: number | undefined) {
    log.push("set a");
    this.stored = value;
  },
};
function pick<T>(value: T): T {
  log.push("object");
  return value;
}
function key(): string {
  log.push("key");
  return "a";
}
function getterOnce(k: number): R {
  if (counted.a == null) {
    const r = read(k);
    if (!("value" in r)) return r;
    counted.a = r.value;
  }
  return { kind: "Ok", value: counted.a };
}
function computedOnce(o: Record<string, number | undefined>, k: number): R {
  const target = pick(o);
  const name = key();
  if (target[name] == null) {
    const r = read(k);
    if (!("value" in r)) return r;
    target[name] = r.value;
  }
  return { kind: "Ok", value: o.a ?? -1 };
}
function or(o: { b: boolean }): boolean {
  o.b ||= log.push("scrutinee") > 0 ? true : false;
  return o.b;
}
function and(o: { c: number }, k: number): R {
  if (o.c) {
    const r = read(k);
    if (!("value" in r)) return r;
    o.c = r.value;
  }
  return { kind: "Ok", value: o.c };
}
console.log(JSON.stringify(nullish({ a: 1 }, -1)), log.splice(0).join());
console.log(JSON.stringify(nullish({}, 2)), log.splice(0).join());
console.log(or({ b: true }), log.splice(0).join());
console.log(or({ b: false }), log.splice(0).join());
console.log(JSON.stringify(and({ c: 0 }, -1)), log.splice(0).join());
console.log(JSON.stringify(and({ c: 1 }, -1)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(-1)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(4)), log.splice(0).join());
console.log(JSON.stringify(getterOnce(-1)), log.splice(0).join());
console.log(JSON.stringify(computedOnce({}, 5)), log.splice(0).join());
console.log(JSON.stringify(computedOnce({ a: 2 }, -5)), log.splice(0).join());
