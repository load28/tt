//// [aLoopHeaderValueIsNotHoistedOutOfTheLoop.tt] ////
declare function id(v: number): number;
let n = 0;
while (id(match (n) { 0 => 1, _ => 0 })) { n = n + 1; }


//// [aLoopHeaderValueIsNotHoistedOutOfTheLoop.ts]
declare function id(v: number): number;
let n = 0;
while (true) {
  let $tt_v0: number;
  const $tt_v1 = (id);
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 0: {
        $tt_v0 = 1;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  if (!($tt_v1($tt_v0))) break; { n = n + 1; }}
