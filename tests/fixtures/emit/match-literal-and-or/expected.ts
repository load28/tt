declare const code: 200 | 201 | 404;

let $tt_v0;
{
  const $tt_m = code;
  switch ($tt_m) {
    case 200: case 201: {
      $tt_v0 = "success";
      break;
    }
    case 404: {
      $tt_v0 = "not found";
      break;
    }
    default: {
      $tt_v0 = "other";
      break;
    }
  }
}
export const status = $tt_v0;
