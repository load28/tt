//// [aLoopHeaderMatchIsEvaluatedEveryIteration.tt] ////

let n = 0;
function next(): number { n = n + 1; return n; }
function id(v: number): number { return v; }
const seen: number[] = [];
while (id(match (next()) { 1 => 1, 2 => 1, _ => 0 })) {
  seen.push(n);
}
console.log(JSON.stringify(seen), n);

export {};


//// [aLoopHeaderMatchIsEvaluatedEveryIteration.ts]

let n = 0;
function next(): number { n = n + 1; return n; }
function id(v: number): number { return v; }
const seen: number[] = [];
while (true) {
  let $tt_v0: number;
  const $tt_v1: typeof id = (id);
  {
    const $tt_m = next();
    switch ($tt_m) {
      case 1: {
        $tt_v0 = 1;
        break;
      }
      case 2: {
        $tt_v0 = 1;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  if (!($tt_v1($tt_v0))) break; {
  seen.push(n);
}}
console.log(JSON.stringify(seen), n);

export {};
