//// [aConditionHoldingAPipelinedMatchIsReadFromItsSlot.tt] ////
const L: unknown[] = [];
function log<T>(x: T): T { L.push(x); return x; }
const h = (x: number) => x;
export function scrutinee() {
  return match ((match (log(1)) { 1 => 1, _ => 2 } |> h) ? match (log(2)) { _ => 1 } : 0) { _ => "ok" };
}
console.log(scrutinee(), L.join());


//// [aConditionHoldingAPipelinedMatchIsReadFromItsSlot.ts]
const L: unknown[] = [];
function log<T>(x: T): T { L.push(x); return x; }
const h = (x: number) => x;
export function scrutinee() {
  let $tt_v0: string;
  {
    let $tt_m_1; let $tt_v4: number;
    do {
      let $tt_v6: number;
      {
        const $tt_m = log(1);
        switch ($tt_m) {
          case 1: {
            $tt_v6 = 1;
            break;
          }
          default: {
            $tt_v6 = 2;
            break;
          }
        }
      }
      $tt_v4 = h($tt_v6);
      break;
    } while (false);
    let $tt_v3: number;
    if ($tt_v4) {
      {
        const $tt_m = log(2);
        switch ($tt_m) {
          default: {
            $tt_v3 = 1;
            break;
          }
        }
      }
    } else {
      $tt_v3 = 0;
    }
    $tt_m_1 = $tt_v3;
    switch ($tt_m_1) {
      default: {
        $tt_v0 = "ok";
        break;
      }
    }
  }
  return $tt_v0;
}
console.log(scrutinee(), L.join());
