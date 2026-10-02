//// [generatedGuardsReachTheHostGlobalsPastAShadowedGlobalThis1.tt] ////

variant O { Some(value: number), None }
function f(o: O, globalThis: unknown) { const Error = 5; return match (o) { Some(value) => value, None => Error }; }
try { f({ kind: "Nope" } as unknown as O, 1); } catch (e) { console.log(e instanceof RangeError, (e as { message: string }).message); }
function g(v: 1 | 2, s: "a" | "b", globalThis: unknown) {
  const JSON = 1, String = 2;
  return match (v) { 1 => JSON, 2 => String } + match (s) { "a" => 1, "b" => 2 };
}
try { g(3 as 1, "a", 1); } catch (e) { console.log((e as { message: string }).message); }
try { g(1, "c" as "a", 1); } catch (e) { console.log((e as { message: string }).message); }

export {};


//// [generatedGuardsReachTheHostGlobalsPastAShadowedGlobalThis1.ts]
const $tt_Error = globalThis.Error;
const $tt_JSON = globalThis.JSON;
const $tt_String = globalThis.String;
function $tt_raise(error: unknown): never { throw error; }
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return $tt_JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return $tt_String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = $tt_JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return $tt_String(value);
}

type O =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const O = {
  Some: (value: number): O => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
function f(o: O, globalThis: unknown) { const Error = 5; let $tt_v0: number;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "Some": {
      const { value } = $tt_m;
      $tt_v0 = value;
      break;
    }
    case "None": {
      $tt_v0 = Error;
      break;
    }
    default: {
      throw new $tt_Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
return $tt_v0; }
try { f({ kind: "Nope" } as unknown as O, 1); } catch (e) { console.log(e instanceof RangeError, (e as { message: string }).message); }
function g(v: 1 | 2, s: "a" | "b", globalThis: unknown) {
  const JSON = 1, String = 2;
  let $tt_subject_1;
  let $tt_subject_2;
  
  return ($tt_subject_1 = v, ($tt_subject_1 === 1) ? JSON : ($tt_subject_1 === 2) ? String : $tt_raise(new $tt_Error("tt match: unexpected literal " + $tt_show($tt_subject_1)))) + ($tt_subject_2 = s, ($tt_subject_2 === "a") ? 1 : ($tt_subject_2 === "b") ? 2 : $tt_raise(new $tt_Error("tt match: unexpected literal " + $tt_show($tt_subject_2))));
}
try { g(3 as 1, "a", 1); } catch (e) { console.log((e as { message: string }).message); }
try { g(1, "c" as "a", 1); } catch (e) { console.log((e as { message: string }).message); }

export {};
