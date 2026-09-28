declare const flag: boolean;
type Item = {run: (x: number) => number};
declare function pair(a: Item, b: Item): void;
let $tt_subject;
let $tt_subject_1;

pair(
  ($tt_subject = flag, ($tt_subject === true) ? ({run: x => x + 1}) : ($tt_subject === false) ? ({run: x => x}) : $tt_raise(new Error("tt match: unexpected literal " + $tt_show($tt_subject)))),
  ($tt_subject_1 = flag, ($tt_subject_1 === true) ? ({run: x => x - 1}) : ($tt_subject_1 === false) ? ({run: x => x}) : $tt_raise(new Error("tt match: unexpected literal " + $tt_show($tt_subject_1)))),
);
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
