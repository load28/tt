//// [runtimeMatchGuardsFallThroughTopToBottom.tt] ////

variant Score {
  Graded(points: number),
  Pending,
}

function grade(s: Score): string {
  return match (s) {
    Graded(points) if points >= 90 => "A",
    Graded(points) if points >= 80 => "B",
    Graded(points) => "F",
    Pending => "-",
  };
}

function tally(s: Score): number {
  return match (s) {
    Graded(points) if points > 0 => {
      const doubled = points * 2;
      return doubled;
    },
    _ => 0,
  };
}

console.log(grade(Score.Graded(95)));
console.log(grade(Score.Graded(85)));
console.log(grade(Score.Graded(10)));
console.log(grade(Score.Pending));
console.log(tally(Score.Graded(3)));
console.log(tally(Score.Graded(-1)));

export {};


//// [runtimeMatchGuardsFallThroughTopToBottom.ts]
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

type Score =
  | { kind: "Graded"; points: number }
  | { kind: "Pending" };
const Score = {
  Graded: (points: number): Score => ({ kind: "Graded", points }),
  Pending: { kind: "Pending" } as const,
};

function grade(s: Score): string {
  let $tt_v0: string;
  {
    const $tt_m = s;
    do {
      if ($tt_m.kind === "Graded") {
        const { points } = $tt_m;
        if (points >= 90) {
          $tt_v0 = "A";
          break;
        }
      }
      if ($tt_m.kind === "Graded") {
        const { points } = $tt_m;
        if (points >= 80) {
          $tt_v0 = "B";
          break;
        }
      }
      if ($tt_m.kind === "Graded") {
        const { points } = $tt_m;
        $tt_v0 = "F";
        break;
      }
      if ($tt_m.kind === "Pending") {
        $tt_v0 = "-";
        break;
      }
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    } while (false);
  }
  return $tt_v0;
}

function tally(s: Score): number {
  let $tt_v1: number;
  {
    const $tt_m = s;
    do {
      if ($tt_m.kind === "Graded") {
        const { points } = $tt_m;
        if (points > 0) {
          {
      const doubled = points * 2;
      $tt_v1 = doubled;
      break;
          }
        }
      }
      $tt_v1 = 0;
      break;
    } while (false);
  }
  return $tt_v1;
}

console.log(grade(Score.Graded(95)));
console.log(grade(Score.Graded(85)));
console.log(grade(Score.Graded(10)));
console.log(grade(Score.Pending));
console.log(tally(Score.Graded(3)));
console.log(tally(Score.Graded(-1)));

export {};
