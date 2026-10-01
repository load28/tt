//// [runGuardRegionsPreserveAllValuesAndShortCircuitEffects.tt] ////

const events: number[] = [];
function mark(n: number) { events.push(n); return n; }
function both(a: boolean, b: boolean) {
  return match (1) {
    1 if (match (mark(1)) { 1 => { const value = a; return value; }, _ => false }) &&
         (match (mark(2)) { 2 => { const value = b; return value; }, _ => false }) => true,
    _ => false
  };
}
function either(a: boolean, b: boolean) {
  return match (1) {
    1 if (match (mark(3)) { 3 => { const value = a; return value; }, _ => false }) ||
         (match (mark(4)) { 4 => { const value = b; return value; }, _ => false }) => true,
    _ => false
  };
}
function choose(a: boolean) {
  return match (1) {
    1 if a ? (match (mark(5)) { 5 => { const value = true; return value; }, _ => false }) :
             (match (mark(6)) { 6 => { const value = false; return value; }, _ => false }) => true,
    _ => false
  };
}
console.log(both(false, true), events.splice(0).join(","));
console.log(both(true, false), events.splice(0).join(","));
console.log(both(true, true), events.splice(0).join(","));
console.log(either(true, false), events.splice(0).join(","));
console.log(either(false, true), events.splice(0).join(","));
console.log(choose(true), events.splice(0).join(","));
console.log(choose(false), events.splice(0).join(","));

export {};


//// [runGuardRegionsPreserveAllValuesAndShortCircuitEffects.ts]

const events: number[] = [];
function mark(n: number) { events.push(n); return n; }
function both(a: boolean, b: boolean) {
  let $tt_v0: boolean;
  {
    const $tt_m = 1;
    do {
      if ($tt_m === 1) {
        let $tt_v1: boolean;
        let $tt_v3: boolean;
        {
          const $tt_m = mark(1);
          switch ($tt_m) {
            case 1: {
              const value = a; $tt_v1 = value; break;
            }
            default: {
              $tt_v1 = false;
              break;
            }
          }
        }
        if ($tt_v1) {
          let $tt_v2: boolean;
          {
            const $tt_m = mark(2);
            switch ($tt_m) {
              case 2: {
                const value = b; $tt_v2 = value; break;
              }
              default: {
                $tt_v2 = false;
                break;
              }
            }
          }
          $tt_v3 = $tt_v1 && $tt_v2;
        } else {
          $tt_v3 = $tt_v1;
        }
        if ($tt_v3) {
          $tt_v0 = true;
          break;
        }
      }
      $tt_v0 = false;
      break;
    } while (false);
  }
  return $tt_v0;
}
function either(a: boolean, b: boolean) {
  let $tt_v4: boolean;
  {
    const $tt_m = 1;
    do {
      if ($tt_m === 1) {
        let $tt_v5: boolean;
        let $tt_v7: boolean;
        {
          const $tt_m = mark(3);
          switch ($tt_m) {
            case 3: {
              const value = a; $tt_v5 = value; break;
            }
            default: {
              $tt_v5 = false;
              break;
            }
          }
        }
        if ($tt_v5) {
          $tt_v7 = $tt_v5;
        } else {
          let $tt_v6: boolean;
          {
            const $tt_m = mark(4);
            switch ($tt_m) {
              case 4: {
                const value = b; $tt_v6 = value; break;
              }
              default: {
                $tt_v6 = false;
                break;
              }
            }
          }
          $tt_v7 = $tt_v5 || $tt_v6;
        }
        if ($tt_v7) {
          $tt_v4 = true;
          break;
        }
      }
      $tt_v4 = false;
      break;
    } while (false);
  }
  return $tt_v4;
}
function choose(a: boolean) {
  let $tt_v8: boolean;
  {
    const $tt_m = 1;
    do {
      if ($tt_m === 1) {
        let $tt_v12: boolean;
        if (a) {
          {
            const $tt_m = mark(5);
            switch ($tt_m) {
              case 5: {
                const value = true; $tt_v12 = value; break;
              }
              default: {
                $tt_v12 = false;
                break;
              }
            }
          }
        } else {
          {
            const $tt_m = mark(6);
            switch ($tt_m) {
              case 6: {
                const value = false; $tt_v12 = value; break;
              }
              default: {
                $tt_v12 = false;
                break;
              }
            }
          }
        }
        if ($tt_v12) {
          $tt_v8 = true;
          break;
        }
      }
      $tt_v8 = false;
      break;
    } while (false);
  }
  return $tt_v8;
}
console.log(both(false, true), events.splice(0).join(","));
console.log(both(true, false), events.splice(0).join(","));
console.log(both(true, true), events.splice(0).join(","));
console.log(either(true, false), events.splice(0).join(","));
console.log(either(false, true), events.splice(0).join(","));
console.log(choose(true), events.splice(0).join(","));
console.log(choose(false), events.splice(0).join(","));

export {};
