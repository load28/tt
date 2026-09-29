import { $tt_ap } from "@tt/runtime";
declare function observe(value: number): number;
let $tt_v1: number;
const $tt_v2 = ($tt_ap(observe(1), ((x: number) => x + 1)));
{
  const $tt_m = 2;
  switch ($tt_m) {
    case 2: {
      const $tt_a0 = { value: 2 };
      $tt_v1 = $tt_a0.value;
      break;
    }
    default: {
      const $tt_a1 = { value: 0 };
      $tt_v1 = $tt_a1.value;
      break;
    }
  }
}
export const values = [($tt_v2), $tt_v1];
let $tt_subject_1;
let $tt_subject_2;

export const calls = Number(($tt_subject_1 = 1, ($tt_subject_1 === 1) ? 1 : 0)) + Number(($tt_subject_2 = 2, ($tt_subject_2 === 2) ? 2 : 0));
let $tt_v9: number;
const $tt_v10 = ((((x: number) => x + 1))(1));
{
  const $tt_m = 2;
  switch ($tt_m) {
    case 2: {
      const $tt_a2 = { value: 2 };
      $tt_v9 = $tt_a2.value;
      break;
    }
    default: {
      const $tt_a3 = { value: 0 };
      $tt_v9 = $tt_a3.value;
      break;
    }
  }
}
export const flows = [$tt_v10, $tt_v9];
