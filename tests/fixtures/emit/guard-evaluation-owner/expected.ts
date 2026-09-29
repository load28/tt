declare function mark(n: number): number;
export function guarded(left: boolean, right: boolean) {
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
              const value = left; $tt_v1 = value; break;
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
                const value = right; $tt_v2 = value; break;
              }
              default: {
                $tt_v2 = false;
                break;
              }
            }
          }
          $tt_v3 = $tt_v2;
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
