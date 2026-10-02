//// [runtimeTupleMatchDispatchesOnTheCombination.tt] ////

variant Conn { Online(latency: number), Offline }
variant Mode { Auto(), Manual(level: number) }

function decide(c: Conn, m: Mode): number {
  return match (c, m) {
    (Online(latency), Auto) if latency < 50 => 10,
    (Online, Auto) => 5,
    (Online, Manual(level)) => level,
    (Offline, _) => 0,
  };
}

console.log(decide(Conn.Online(10), Mode.Auto()));
console.log(decide(Conn.Online(80), Mode.Auto()));
console.log(decide(Conn.Online(10), Mode.Manual(7)));
console.log(decide(Conn.Offline, Mode.Auto()));

export {};


//// [runtimeTupleMatchDispatchesOnTheCombination.ts]
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

type Conn =
  | { kind: "Online"; latency: number }
  | { kind: "Offline" };
const Conn = {
  Online: (latency: number): Conn => ({ kind: "Online", latency }),
  Offline: { kind: "Offline" } as const,
};
type Mode =
  | { kind: "Auto" }
  | { kind: "Manual"; level: number };
const Mode = {
  Auto: (): Mode => ({ kind: "Auto" }),
  Manual: (level: number): Mode => ({ kind: "Manual", level }),
};

function decide(c: Conn, m: Mode): number {
  let $tt_v0: number;
  {
    const $tt_m0 = c;
    const $tt_m1 = m;
    do {
      if ($tt_m0.kind === "Online" && $tt_m1.kind === "Auto") {
        const { latency } = $tt_m0;
        if (latency < 50) {
          $tt_v0 = 10;
          break;
        }
      }
      if ($tt_m0.kind === "Online" && $tt_m1.kind === "Auto") {
        $tt_v0 = 5;
        break;
      }
      if ($tt_m0.kind === "Online" && $tt_m1.kind === "Manual") {
        const { level } = $tt_m1;
        $tt_v0 = level;
        break;
      }
      if ($tt_m0.kind === "Offline") {
        $tt_v0 = 0;
        break;
      }
      throw new Error("tt match: unexpected case " + "[" + $tt_show($tt_m0) + "," + $tt_show($tt_m1) + "]");
    } while (false);
  }
  return $tt_v0;
}

console.log(decide(Conn.Online(10), Mode.Auto()));
console.log(decide(Conn.Online(80), Mode.Auto()));
console.log(decide(Conn.Online(10), Mode.Manual(7)));
console.log(decide(Conn.Offline, Mode.Auto()));

export {};
