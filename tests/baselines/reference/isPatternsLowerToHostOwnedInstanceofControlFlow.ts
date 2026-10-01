//// [isPatternsLowerToHostOwnedInstanceofControlFlow.tt] ////
const msg = match (err) {
is SyntaxError { message } if message.length > 0 => `syntax: ${message}`,
is RangeError | is TypeError => "bad value",
is Error { message: detail } => detail,
_ => String(err),
};


//// [isPatternsLowerToHostOwnedInstanceofControlFlow.ts]
let $tt_v0$msg: string;
{
  const $tt_m = err;
  do {
    if ($tt_m instanceof SyntaxError) {
      const { message } = $tt_m;
      if (message.length > 0) {
        $tt_v0$msg = `syntax: ${message}`;
        break;
      }
    }
    if ($tt_m instanceof RangeError || $tt_m instanceof TypeError) {
      $tt_v0$msg = "bad value";
      break;
    }
    if ($tt_m instanceof Error) {
      const { message: detail } = $tt_m;
      $tt_v0$msg = detail;
      break;
    }
    $tt_v0$msg = String(err);
    break;
  } while (false);
}
const msg = $tt_v0$msg;
