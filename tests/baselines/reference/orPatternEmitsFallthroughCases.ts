//// [orPatternEmitsFallthroughCases.tt] ////

variant Key { Enter(), Escape, Tab, Char(ch: string) }
const action = match (key) {
  Enter => "submit",
  Escape | Tab => "cancel",
  Char(ch) => "type:" + ch,
};


//// [orPatternEmitsFallthroughCases.ts]
var $tt_show: (value: unknown) => string = function (value) {
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
};

type Key =
  | { kind: "Enter" }
  | { kind: "Escape" }
  | { kind: "Tab" }
  | { kind: "Char"; ch: string };
const Key = {
  Enter: (): Key => ({ kind: "Enter" }),
  Escape: { kind: "Escape" } as const,
  Tab: { kind: "Tab" } as const,
  Char: (ch: string): Key => ({ kind: "Char", ch }),
};
let $tt_v0$action: string;
{
  const $tt_m = key;
  switch ($tt_m.kind) {
    case "Enter": {
      $tt_v0$action = "submit";
      break;
    }
    case "Escape": case "Tab": {
      $tt_v0$action = "cancel";
      break;
    }
    case "Char": {
      const { ch } = $tt_m;
      $tt_v0$action = "type:" + ch;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const action = $tt_v0$action;
