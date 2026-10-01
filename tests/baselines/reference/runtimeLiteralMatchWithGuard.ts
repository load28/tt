//// [runtimeLiteralMatchWithGuard.tt] ////

function classify(code: number, retry: boolean) {
  return match (code) {
    500 if retry => "retrying",
    500 => "failed",
    _ => "ok",
  };
}

console.log(classify(500, true), classify(500, false), classify(200, true));

export {};


//// [runtimeLiteralMatchWithGuard.ts]

function classify(code: number, retry: boolean) {
  let $tt_v0: string;
  {
    const $tt_m = code;
    do {
      if ($tt_m === 500) {
        if (retry) {
          $tt_v0 = "retrying";
          break;
        }
      }
      if ($tt_m === 500) {
        $tt_v0 = "failed";
        break;
      }
      $tt_v0 = "ok";
      break;
    } while (false);
  }
  return $tt_v0;
}

console.log(classify(500, true), classify(500, false), classify(200, true));

export {};
