declare const error: unknown;

let $tt_v0: string;
{
  const $tt_m = error;
  do {
    if ($tt_m instanceof SyntaxError) {
      const { message } = $tt_m;
      if (message.length > 0) {
        const $tt_a0 = { value: `syntax: ${message}` };
        $tt_v0 = $tt_a0.value;
        break;
      }
    }
    if ($tt_m instanceof RangeError || $tt_m instanceof TypeError) {
      const $tt_a1 = { value: "bad value" };
      $tt_v0 = $tt_a1.value;
      break;
    }
    if ($tt_m instanceof Error) {
      const { message: detail } = $tt_m;
      const $tt_a2 = { value: detail };
      $tt_v0 = $tt_a2.value;
      break;
    }
    const $tt_a3 = { value: String(error) };
    $tt_v0 = $tt_a3.value;
    break;
  } while (false);
}
export const message = $tt_v0;
