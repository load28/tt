//// [runtimeAnOptionalMemberCallIsSkippedOnlyWhenItsReceiverIsNullish.tt] ////

type M = { base?: number; m(v: number): number };
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
function arg(tag: string): number { trace.push(tag); return 1; }
function read(tag: string): R { trace.push(tag); return { kind: "Ok", value: 2 }; }
const live: M = { base: 7, m(v: number): number { trace.push("this:" + (this === live)); return (this.base ?? 0) + v; } };
const missing = {} as M;
function attempt(f: () => unknown): string {
  try { return String(f()); } catch (e) { return (e as Error).constructor.name; }
}
function viaTry(o: M | null, tag: string): R {
  const called = o?.m(try read(tag));
  return { kind: "Ok", value: called ?? -1 };
}
function each(o: M | null, tag: string): string[] {
  return [
    attempt(() => o?.m(match (arg(tag + ":call")) { 1 => 1, _ => 0 })),
    attempt(() => o?.m?.(match (arg(tag + ":both")) { 1 => 1, _ => 0 })),
    attempt(() => JSON.stringify(viaTry(o, tag + ":try"))),
    attempt(() => o |> ?.m(match (arg(tag + ":pipe")) { 1 => 1, _ => 0 })),
  ];
}
console.log(JSON.stringify([each(live, "live"), each(null, "null"), each(missing, "missing")]));
console.log(JSON.stringify(trace));

export {};


//// [runtimeAnOptionalMemberCallIsSkippedOnlyWhenItsReceiverIsNullish.ts]

type M = { base?: number; m(v: number): number };
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
function arg(tag: string): number { trace.push(tag); return 1; }
function read(tag: string): R { trace.push(tag); return { kind: "Ok", value: 2 }; }
const live: M = { base: 7, m(v: number): number { trace.push("this:" + (this === live)); return (this.base ?? 0) + v; } };
const missing = {} as M;
function attempt(f: () => unknown): string {
  try { return String(f()); } catch (e) { return (e as Error).constructor.name; }
}
function viaTry(o: M | null, tag: string): R {
  let $tt_v2: (number) | (undefined);
  if (o != null) {
    let $tt_v0: number;
    const $tt_t0 = read(tag);
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v0 = $tt_t0.value;
    $tt_v2 = o?.m($tt_v0);
  } else {
    $tt_v2 = undefined;
  }
  
  const called = $tt_v2;
  return { kind: "Ok", value: called ?? -1 };
}
function each(o: M | null, tag: string): string[] {
  return [
    attempt(() => {
      let $tt_v5: (number) | (undefined);
      if (o != null) {
        {
          const $tt_m = arg(tag + ":call");
          switch ($tt_m) {
            case 1: {
              $tt_v5 = o?.m(1);
              break;
            }
            default: {
              $tt_v5 = o?.m(0);
              break;
            }
          }
        }
      } else {
        $tt_v5 = undefined;
      }
      
      return $tt_v5;
    }),
    attempt(() => {
      let $tt_v8: (number) | (undefined);
      const $tt_v7 = (o?.m);
      if ($tt_v7 != null) {
        {
          const $tt_m = arg(tag + ":both");
          switch ($tt_m) {
            case 1: {
              $tt_v8 = $tt_v7.call(o, 1);
              break;
            }
            default: {
              $tt_v8 = $tt_v7.call(o, 0);
              break;
            }
          }
        }
      } else {
        $tt_v8 = undefined;
      }
      
      return $tt_v8;
    }),
    attempt(() => JSON.stringify(viaTry(o, tag + ":try"))),
    attempt(() => {
      let $tt_v9: number | undefined;
      do {
        const $tt_v14 = o;
        let $tt_v13: (number) | (undefined);
        const $tt_v12 = ($tt_v14);
        if ($tt_v12 != null) {
          {
            const $tt_m = arg(tag + ":pipe");
            switch ($tt_m) {
              case 1: {
                $tt_v13 = $tt_v12?.m(1);
                break;
              }
              default: {
                $tt_v13 = $tt_v12?.m(0);
                break;
              }
            }
          }
        } else {
          $tt_v13 = undefined;
        }
        $tt_v9 = $tt_v13;
        break;
      } while (false);
      return $tt_v9;
    }),
  ];
}
console.log(JSON.stringify([each(live, "live"), each(null, "null"), each(missing, "missing")]));
console.log(JSON.stringify(trace));

export {};
