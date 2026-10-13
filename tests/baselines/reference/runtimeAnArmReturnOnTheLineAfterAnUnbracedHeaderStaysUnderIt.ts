//// [runtimeAnArmReturnOnTheLineAfterAnUnbracedHeaderStaysUnderIt.tt] ////

function pick(v: number, c: boolean): number | undefined {
  return match (v) {
    1 => {
      if (c)
        return 10;
      if (!c)
        return;
      return 20;
    },
    _ => 0,
  };
}
function twice(x: number): number {
  return x * 2;
}
function nested(v: number, c: boolean): number {
  return match (v) {
    1 => {
      if (c)
        return twice(match (v) { 1 => 3, _ => 4 })
      ;return 30;
    },
    _ => 0,
  };
}
console.log(pick(1, true), pick(1, false), pick(2, true));
console.log(nested(1, true), nested(1, false), nested(2, false));

export {};


//// [runtimeAnArmReturnOnTheLineAfterAnUnbracedHeaderStaysUnderIt.ts]

function pick(v: number, c: boolean): number | undefined {
  let $tt_v0: number | undefined;
  {
    const $tt_m = v;
    switch ($tt_m) {
      case 1: {
        if (c)
          { $tt_v0 = 10;
          break; }
      if (!c)
          { $tt_v0 = undefined;
          break; }
        $tt_v0 = 20;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  return $tt_v0;
}
function twice(x: number): number {
  return x * 2;
}
function nested(v: number, c: boolean): number {
  let $tt_v1: number;
  {
    const $tt_m = v;
    switch ($tt_m) {
      case 1: {
        if (c)
        {
          let $tt_v2: number;
          const $tt_v3: typeof twice = (twice);
          {
            const $tt_m = v;
            switch ($tt_m) {
              case 1: $tt_v2 = 0; break;
              default: $tt_v2 = 1; break;
            }
          }
            { $tt_v1 = $tt_v3(($tt_v2 === 0 ? 3 : 4));
          break; }
        }$tt_v1 = 30; break;
      }
      default: {
        $tt_v1 = 0;
        break;
      }
    }
  }
  return $tt_v1;
}
console.log(pick(1, true), pick(1, false), pick(2, true));
console.log(nested(1, true), nested(1, false), nested(2, false));

export {};
