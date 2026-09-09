declare const error: unknown;

let $tt_v0;
{
  const $tt_m = error;
  do {
    if ($tt_m instanceof SyntaxError) {
      const { message } = $tt_m;
      if (message.length > 0) {
        $tt_v0 = `syntax: ${message}`;
        break;
      }
    }
    if ($tt_m instanceof RangeError || $tt_m instanceof TypeError) {
      $tt_v0 = "bad value";
      break;
    }
    if ($tt_m instanceof Error) {
      const { message: detail } = $tt_m;
      $tt_v0 = detail;
      break;
    }
    $tt_v0 = String(error);
    break;
  } while (false);
}
export const message = $tt_v0;
