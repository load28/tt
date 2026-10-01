//// [anUnexpectedCaseReportsItsOwnMatchSubject.tt] ////

const pick = (s: string) => s as "a" | "b";
function outer(s: string) {
  return match (match (pick(s)) { "a" => pick("z"), "b" => pick("b") }) { "a" => 1, "b" => 2 };
}
try { outer("a"); } catch (error) { console.log((error as Error).message); }
console.log(outer("b"));

export {};


//// [anUnexpectedCaseReportsItsOwnMatchSubject.ts]
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

const pick = (s: string) => s as "a" | "b";
function outer(s: string) {
  let $tt_v0: number;
  {
    let $tt_m_1; {
      const $tt_m = pick(s);
      switch ($tt_m) {
        case "a": {
          $tt_m_1 = pick("z");
          break;
        }
        case "b": {
          $tt_m_1 = pick("b");
          break;
        }
        default: {
          throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
        }
      }
    }
    switch ($tt_m_1) {
      case "a": {
        $tt_v0 = 1;
        break;
      }
      case "b": {
        $tt_v0 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m_1));
      }
    }
  }
  return $tt_v0;
}
try { outer("a"); } catch (error) { console.log((error as Error).message); }
console.log(outer("b"));

export {};
