//// [runtimeLiteralMatchEvaluatesTheScrutineeOnce.tt] ////

let calls = 0;
function getValue(): string {
  calls += 1;
  return "b";
}

const picked = match (getValue()) {
  "a" => 1,
  "b" => 2,
  _ => 3,
};
console.log(picked, calls);

export {};


//// [runtimeLiteralMatchEvaluatesTheScrutineeOnce.ts]

let calls = 0;
function getValue(): string {
  calls += 1;
  return "b";
}

let $tt_v0: number;
{
  const $tt_m = getValue();
  switch ($tt_m) {
    case "a": {
      $tt_v0 = 1;
      break;
    }
    case "b": {
      $tt_v0 = 2;
      break;
    }
    default: {
      $tt_v0 = 3;
      break;
    }
  }
}
const picked = $tt_v0;
console.log(picked, calls);

export {};
