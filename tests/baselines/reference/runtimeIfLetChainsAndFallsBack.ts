//// [runtimeIfLetChainsAndFallsBack.tt] ////

variant Opt { Some(value: number), None }

function pick(a: Opt, b: Opt): number {
  let out = -1;
  if let Some(value) = a {
    out = value;
  } else if let Some(value) = b {
    out = value * 10;
  } else {
    out = 0;
  }
  return out;
}

console.log(pick(Opt.Some(1), Opt.Some(2)));
console.log(pick(Opt.None, Opt.Some(2)));
console.log(pick(Opt.None, Opt.None));

export {};


//// [runtimeIfLetChainsAndFallsBack.ts]

type Opt =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const Opt = {
  Some: (value: number): Opt => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};

function pick(a: Opt, b: Opt): number {
  let out = -1;
  {
    const $tt_t0 = a;
    if ($tt_t0.kind === "Some") {
      const { value } = $tt_t0;
      out = value;
    } else {
      const $tt_t1 = b;
      if ($tt_t1.kind === "Some") {
        const { value } = $tt_t1;
        out = value * 10;
      } else {
        out = 0;
      }
    }
  }
  return out;
}

console.log(pick(Opt.Some(1), Opt.Some(2)));
console.log(pick(Opt.None, Opt.Some(2)));
console.log(pick(Opt.None, Opt.None));

export {};
