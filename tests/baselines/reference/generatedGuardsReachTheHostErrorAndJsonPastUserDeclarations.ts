//// [generatedGuardsReachTheHostErrorAndJsonPastUserDeclarations.tt] ////

variant Error { Bad(msg: string), Worse }
function m(x: Error) { return match (x) { Bad(msg) => msg, Worse => "w" }; }
try { m({ kind: "Nope" } as unknown as Error); } catch (e) { console.log((e as globalThis.Error).message); }
const JSON = 1;
const k = (v: "a" | "b") => match (v) { "a" => 1, "b" => JSON };
try { k("c" as unknown as "a"); } catch (e) { console.log((e as globalThis.Error).message); }

export {};


//// [generatedGuardsReachTheHostErrorAndJsonPastUserDeclarations.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return globalThis.JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = globalThis.JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}

type Error =
  | { kind: "Bad"; msg: string }
  | { kind: "Worse" };
const Error = {
  Bad: (msg: string): Error => ({ kind: "Bad", msg }),
  Worse: { kind: "Worse" } as const,
};
function m(x: Error) { let $tt_v0: string;
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "Bad": {
      const { msg } = $tt_m;
      $tt_v0 = msg;
      break;
    }
    case "Worse": {
      $tt_v0 = "w";
      break;
    }
    default: {
      throw new globalThis.Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
return $tt_v0; }
try { m({ kind: "Nope" } as unknown as Error); } catch (e) { console.log((e as globalThis.Error).message); }
const JSON = 1;
const k = (v: "a" | "b") => {
  let $tt_v1: number;
  {
    const $tt_m = v;
    switch ($tt_m) {
      case "a": {
        $tt_v1 = 1;
        break;
      }
      case "b": {
        $tt_v1 = JSON;
        break;
      }
      default: {
        throw new globalThis.Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v1;
};
try { k("c" as unknown as "a"); } catch (e) { console.log((e as globalThis.Error).message); }

export {};
