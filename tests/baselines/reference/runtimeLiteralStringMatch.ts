//// [runtimeLiteralStringMatch.tt] ////

type Direction = "north" | "south" | "east" | "west";

function short(dir: Direction) {
  return match (dir) {
    "north" => "N",
    "south" => "S",
    "east" => "E",
    "west" => "W",
  };
}

console.log(short("north"), short("south"), short("east"), short("west"));

export {};


//// [runtimeLiteralStringMatch.ts]
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

type Direction = "north" | "south" | "east" | "west";

function short(dir: Direction) {
  let $tt_v0: string;
  {
    const $tt_m = dir;
    switch ($tt_m) {
      case "north": {
        $tt_v0 = "N";
        break;
      }
      case "south": {
        $tt_v0 = "S";
        break;
      }
      case "east": {
        $tt_v0 = "E";
        break;
      }
      case "west": {
        $tt_v0 = "W";
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}

console.log(short("north"), short("south"), short("east"), short("west"));

export {};
