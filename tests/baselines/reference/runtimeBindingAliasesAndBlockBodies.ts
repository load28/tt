//// [runtimeBindingAliasesAndBlockBodies.tt] ////

variant Msg {
  Quit,
  Move(x: number, y: number),
  Write(text: string),
}

function describe(m: Msg): string {
  return match (m) {
    Move(x: px, y: py) => {
      const sum = px + py;
      return "move:" + sum;
    },
    Write(text) => "write:" + text,
    Quit => "quit",
  };
}

console.log(describe(Msg.Move(2, 3)));
console.log(describe(Msg.Write("hi")));
console.log(describe(Msg.Quit));

export {};


//// [runtimeBindingAliasesAndBlockBodies.ts]
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

type Msg =
  | { kind: "Quit" }
  | { kind: "Move"; x: number; y: number }
  | { kind: "Write"; text: string };
const Msg = {
  Quit: { kind: "Quit" } as const,
  Move: (x: number, y: number): Msg => ({ kind: "Move", x, y }),
  Write: (text: string): Msg => ({ kind: "Write", text }),
};

function describe(m: Msg): string {
  let $tt_v0: string;
  {
    const $tt_m = m;
    switch ($tt_m.kind) {
      case "Move": {
        const { x: px, y: py } = $tt_m;
        const sum = px + py;
        $tt_v0 = "move:" + sum;
        break;
      }
      case "Write": {
        const { text } = $tt_m;
        $tt_v0 = "write:" + text;
        break;
      }
      case "Quit": {
        $tt_v0 = "quit";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}

console.log(describe(Msg.Move(2, 3)));
console.log(describe(Msg.Write("hi")));
console.log(describe(Msg.Quit));

export {};
