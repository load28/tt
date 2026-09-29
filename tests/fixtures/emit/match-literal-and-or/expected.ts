declare const code: 200 | 201 | 404;

let $tt_v0: string;
{
  const $tt_m = code;
  switch ($tt_m) {
    case 200: case 201: {
      const $tt_a0 = { value: "success" };
      $tt_v0 = $tt_a0.value;
      break;
    }
    case 404: {
      const $tt_a1 = { value: "not found" };
      $tt_v0 = $tt_a1.value;
      break;
    }
    default: {
      const $tt_a2 = { value: "other" };
      $tt_v0 = $tt_a2.value;
      break;
    }
  }
}
export const status = $tt_v0;
