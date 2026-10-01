//// [literalPatternsNestInsideArmBodies.tt] ////

const v = match (a) {
  "x" => match (b) { 1 => "one", _ => "other" },
  _ => "none",
};


//// [literalPatternsNestInsideArmBodies.ts]

let $tt_v0$v: string;
{
  const $tt_m = a;
  switch ($tt_m) {
    case "x": {
      {
        const $tt_m = b;
        switch ($tt_m) {
          case 1: {
            $tt_v0$v = "one";
            break;
          }
          default: {
            $tt_v0$v = "other";
            break;
          }
        }
      }
      break;
    }
    default: {
      $tt_v0$v = "none";
      break;
    }
  }
}
const v = $tt_v0$v;
