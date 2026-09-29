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
              const value = left; const $tt_a0 = { value: value }; $tt_v1 = $tt_a0.value; break;
            }
            default: {
              const $tt_a1 = { value: false };
              $tt_v1 = $tt_a1.value;
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
                const value = right; const $tt_a2 = { value: value }; $tt_v2 = $tt_a2.value; break;
              }
              default: {
                const $tt_a3 = { value: false };
                $tt_v2 = $tt_a3.value;
                break;
              }
            }
          }
          const $tt_a4 = { value: $tt_v2 };
          $tt_v3 = $tt_a4.value;
        } else {
          const $tt_a5 = { value: $tt_v1 };
          $tt_v3 = $tt_a5.value;
        }
        if ($tt_v3) {
          const $tt_a6 = { value: true };
          $tt_v0 = $tt_a6.value;
          break;
        }
      }
      const $tt_a7 = { value: false };
      $tt_v0 = $tt_a7.value;
      break;
    } while (false);
  }
  return $tt_v0;
}
