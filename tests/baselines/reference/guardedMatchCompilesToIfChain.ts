//// [guardedMatchCompilesToIfChain.tt] ////

variant Score { Graded(points: number), Pending }
const grade = match (s) {
  Graded(points) if points >= 90 => "A",
  Graded(points) => "F",
  Pending => "-",
};


//// [guardedMatchCompilesToIfChain.ts]
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

type Score =
  | { kind: "Graded"; points: number }
  | { kind: "Pending" };
const Score = {
  Graded: (points: number): Score => ({ kind: "Graded", points }),
  Pending: { kind: "Pending" } as const,
};
let $tt_v0$grade: string;
{
  const $tt_m = s;
  do {
    if ($tt_m.kind === "Graded") {
      const { points } = $tt_m;
      if (points >= 90) {
        $tt_v0$grade = "A";
        break;
      }
    }
    if ($tt_m.kind === "Graded") {
      const { points } = $tt_m;
      $tt_v0$grade = "F";
      break;
    }
    if ($tt_m.kind === "Pending") {
      $tt_v0$grade = "-";
      break;
    }
    throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  } while (false);
}
const grade = $tt_v0$grade;
