//// [literalOrPatternSharesOneBodyViaFallthrough.tt] ////

const kind = match (code) {
  200 | 201 | 204 => "success",
  400 | 404 => "client error",
  _ => "unknown",
};


//// [literalOrPatternSharesOneBodyViaFallthrough.ts]

let $tt_v0$kind: string;
{
  const $tt_m = code;
  switch ($tt_m) {
    case 200: case 201: case 204: {
      $tt_v0$kind = "success";
      break;
    }
    case 400: case 404: {
      $tt_v0$kind = "client error";
      break;
    }
    default: {
      $tt_v0$kind = "unknown";
      break;
    }
  }
}
const kind = $tt_v0$kind;
