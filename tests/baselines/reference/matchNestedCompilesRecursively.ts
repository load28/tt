//// [matchNestedCompilesRecursively.tt] ////

const r = match (a) {
  X(inner) => match (inner) { Y => 1, _ => 2 },
  _ => 0,
};


//// [matchNestedCompilesRecursively.ts]

let $tt_v0$r: number;
{
  const $tt_m = a;
  switch ($tt_m.kind) {
    case "X": {
      const { inner } = $tt_m;
      {
        const $tt_m = inner;
        switch ($tt_m.kind) {
          case "Y": {
            $tt_v0$r = 1;
            break;
          }
          default: {
            $tt_v0$r = 2;
            break;
          }
        }
      }
      break;
    }
    default: {
      $tt_v0$r = 0;
      break;
    }
  }
}
const r = $tt_v0$r;
