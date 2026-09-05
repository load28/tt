declare const flag: boolean;
type Item = {run: (x: number) => number};
declare function pair(a: Item, b: Item): void;
let $tt_subject;
let $tt_subject_1;

pair(
  ($tt_subject = flag, ($tt_subject === true) ? ({run: x => x + 1}) : ($tt_subject === false) ? ({run: x => x}) : $tt_raise(new Error("tt match: unexpected literal " + JSON.stringify($tt_subject)))),
  ($tt_subject_1 = flag, ($tt_subject_1 === true) ? ({run: x => x - 1}) : ($tt_subject_1 === false) ? ({run: x => x}) : $tt_raise(new Error("tt match: unexpected literal " + JSON.stringify($tt_subject_1)))),
);
function $tt_raise(error: unknown): never { throw error; }
