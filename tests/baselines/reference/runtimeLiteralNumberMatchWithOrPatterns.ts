//// [runtimeLiteralNumberMatchWithOrPatterns.tt] ////

function status(code: 200 | 201 | 404 | 500) {
  return match (code) {
    200 | 201 => "success",
    404 => "not found",
    500 => "server error",
  };
}

console.log(status(200), status(201), status(404), status(500));

export {};


//// [runtimeLiteralNumberMatchWithOrPatterns.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}

function status(code: 200 | 201 | 404 | 500) {
  let $tt_v0: string;
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
      case 500: {
        $tt_v0 = "server error";
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}

console.log(status(200), status(201), status(404), status(500));

export {};
