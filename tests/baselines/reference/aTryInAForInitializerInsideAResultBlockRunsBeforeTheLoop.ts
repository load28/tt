//// [aTryInAForInitializerInsideAResultBlockRunsBeforeTheLoop.tt] ////
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const ok = (value: number): R<number> => ({ kind: "Ok", value });
const err = (error: string): R<number> => ({ kind: "Err", error });

function sum(start: R<number>): R<number> {
    return result {
        let total = 0;
        for (let i = try start; i < 3; i++) { total += i; }
        return total;
    };
}

console.log(JSON.stringify(sum(ok(0))), JSON.stringify(sum(err("no start"))));
export {};


//// [aTryInAForInitializerInsideAResultBlockRunsBeforeTheLoop.ts]
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const ok = (value: number): R<number> => ({ kind: "Ok", value });
const err = (error: string): R<number> => ({ kind: "Err", error });

function sum(start: R<number>): R<number> {
    let $tt_v0: R<number>;
    $tt_v0: {
      let total = 0;
        const $tt_t0 = start;
        if (!("value" in $tt_t0)) {
          $tt_v0 = $tt_t0;
          break $tt_v0;
        }
        for (let i = $tt_t0.value; i < 3; i++) { total += i; }
        {
          $tt_v0 = { kind: "Ok" as const, value: total };
          break $tt_v0;
        }
    }
    return $tt_v0;
}

console.log(JSON.stringify(sum(ok(0))), JSON.stringify(sum(err("no start"))));
export {};
