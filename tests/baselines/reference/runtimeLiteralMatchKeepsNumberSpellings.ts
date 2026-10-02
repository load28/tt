//// [runtimeLiteralMatchKeepsNumberSpellings.tt] ////

function pick(n: number) {
  return match (n) {
    0xff => "hex",
    1_000 => "sep",
    1.5e2 => "exp",
    -1 => "neg",
    _ => "other",
  };
}

console.log(pick(255), pick(1000), pick(150), pick(-1), pick(0));

export {};


//// [runtimeLiteralMatchKeepsNumberSpellings.ts]

function pick(n: number) {
  let $tt_v0: string;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 0xff: {
        $tt_v0 = "hex";
        break;
      }
      case 1_000: {
        $tt_v0 = "sep";
        break;
      }
      case 1.5e2: {
        $tt_v0 = "exp";
        break;
      }
      case -1: {
        $tt_v0 = "neg";
        break;
      }
      default: {
        $tt_v0 = "other";
        break;
      }
    }
  }
  return $tt_v0;
}

console.log(pick(255), pick(1000), pick(150), pick(-1), pick(0));

export {};
