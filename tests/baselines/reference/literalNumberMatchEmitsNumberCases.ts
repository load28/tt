//// [literalNumberMatchEmitsNumberCases.tt] ////

const message = match (status) {
  200 => "ok",
  404 => "not found",
  500 => "error",
  _ => "unknown",
};


//// [literalNumberMatchEmitsNumberCases.ts]

let $tt_v0$message: string;
{
  const $tt_m = status;
  switch ($tt_m) {
    case 200: {
      $tt_v0$message = "ok";
      break;
    }
    case 404: {
      $tt_v0$message = "not found";
      break;
    }
    case 500: {
      $tt_v0$message = "error";
      break;
    }
    default: {
      $tt_v0$message = "unknown";
      break;
    }
  }
}
const message = $tt_v0$message;
