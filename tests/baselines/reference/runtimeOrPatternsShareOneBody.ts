//// [runtimeOrPatternsShareOneBody.tt] ////

variant Key {
  Enter(),
  Escape,
  Tab,
  Char(ch: string),
}

function action(k: Key): string {
  return match (k) {
    Enter => "submit",
    Escape | Tab => "cancel",
    Char(ch) => "type:" + ch,
  };
}

console.log(action(Key.Enter()));
console.log(action(Key.Escape));
console.log(action(Key.Tab));
console.log(action(Key.Char("z")));

export {};


//// [runtimeOrPatternsShareOneBody.ts]
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

function action(k: Key): string {
  let $tt_v0: string;
  {
    const $tt_m = k;
    switch ($tt_m.kind) {
      case "Enter": {
        $tt_v0 = "submit";
        break;
      }
      case "Escape": case "Tab": {
        $tt_v0 = "cancel";
        break;
      }
      case "Char": {
        const { ch } = $tt_m;
        $tt_v0 = "type:" + ch;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}

console.log(action(Key.Enter()));
console.log(action(Key.Escape));
console.log(action(Key.Tab));
console.log(action(Key.Char("z")));

export {};
