//// [runtimeMatchSuspendsItsGeneratorFromTheSubjectAndGuard.tt] ////

variant S { A(n: number), B }

function* subject(): Generator<string, number, S> {
  const r = match (yield "subject") { A(n) => n, B => 0 };
  return r;
}

function* guard(s: S): Generator<number, string, number> {
  const r = match (s) { A(n) if (yield n) === 1 => `one ${n}`, _ => "other" };
  return r;
}

function* cast(): Generator<number, number, unknown> {
  const r = match ((yield 1) as S) {
    A(n) => match ((yield n) as S) { A(n: m) => n + m, B => n },
    B => 0,
  };
  return r;
}

class Base { start() { return 7; } }
class Derived extends Base {
  scale = 10;
  *run(): Generator<number, number, S> {
    return match (yield super.start()) { A(n) => n * this.scale, B => -1 };
  }
}

async function* later(): AsyncGenerator<number, number, Promise<S>> {
  const r = match (await (yield 1)) { A(n) => n, B => 0 };
  return r;
}

const drive = <Y, R, N>(g: Generator<Y, R, N>, sent: N[]) => {
  const seen: unknown[] = [JSON.stringify(g.next().value)];
  for (const value of sent) seen.push(JSON.stringify(g.next(value).value));
  return seen.join(" ");
};

console.log(drive(subject(), [S.A(4)]), drive(subject(), [S.B]));
console.log(drive(guard(S.A(3)), [1]), drive(guard(S.A(3)), [2]), drive(guard(S.B), []));
console.log(drive(cast(), [S.A(2), S.A(5)]), drive(cast(), [S.A(2), S.B]), drive(cast(), [S.B]));
console.log(drive(new Derived().run(), [S.A(3)]));
const iterator = later();
iterator.next().then((first) =>
  iterator.next(Promise.resolve(S.A(9))).then((last) => console.log(first.value, last.value)),
);

export {};


//// [runtimeMatchSuspendsItsGeneratorFromTheSubjectAndGuard.ts]
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

type S =
  | { kind: "A"; n: number }
  | { kind: "B" };
const S = {
  A: (n: number): S => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};

function* subject(): Generator<string, number, S> {
  let $tt_v0: number;
  {
    const $tt_m = yield "subject";
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = n;
        break;
      }
      case "B": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const r = $tt_v0;
  return r;
}

function* guard(s: S): Generator<number, string, number> {
  let $tt_v1: string;
  {
    const $tt_m = s;
    do {
      if ($tt_m.kind === "A") {
        const { n } = $tt_m;
        if ((yield n) === 1) {
          $tt_v1 = `one ${n}`;
          break;
        }
      }
      $tt_v1 = "other";
      break;
    } while (false);
  }
  const r = $tt_v1;
  return r;
}

function* cast(): Generator<number, number, unknown> {
  let $tt_v2: number;
  {
    const $tt_m = (yield 1) as S;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        {
          const $tt_m = (yield n) as S;
          switch ($tt_m.kind) {
            case "A": {
              const { n: m } = $tt_m;
              $tt_v2 = n + m;
              break;
            }
            case "B": {
              $tt_v2 = n;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        break;
      }
      case "B": {
        $tt_v2 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const r = $tt_v2;
  return r;
}

class Base { start() { return 7; } }
class Derived extends Base {
  scale = 10;
  *run(): Generator<number, number, S> {
    let $tt_v3: number;
    {
      const $tt_m = yield super.start();
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v3 = n * this.scale;
          break;
        }
        case "B": {
          $tt_v3 = -1;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    return $tt_v3;
  }
}

async function* later(): AsyncGenerator<number, number, Promise<S>> {
  let $tt_v4: number;
  {
    const $tt_m = await (yield 1);
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v4 = n;
        break;
      }
      case "B": {
        $tt_v4 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const r = $tt_v4;
  return r;
}

const drive = <Y, R, N>(g: Generator<Y, R, N>, sent: N[]) => {
  const seen: unknown[] = [JSON.stringify(g.next().value)];
  for (const value of sent) seen.push(JSON.stringify(g.next(value).value));
  return seen.join(" ");
};

console.log(drive(subject(), [S.A(4)]), drive(subject(), [S.B]));
console.log(drive(guard(S.A(3)), [1]), drive(guard(S.A(3)), [2]), drive(guard(S.B), []));
console.log(drive(cast(), [S.A(2), S.A(5)]), drive(cast(), [S.A(2), S.B]), drive(cast(), [S.B]));
console.log(drive(new Derived().run(), [S.A(3)]));
const iterator = later();
iterator.next().then((first) =>
  iterator.next(Promise.resolve(S.A(9))).then((last) => console.log(first.value, last.value)),
);

export {};
