//// [ifLetBindingsStayNarrowedInsideClosures.tt] ////

variant Opt { Some(value: string), None }
function f(o: Opt, xs: number[]): string[] {
  const collected: string[] = [];
  if let Some(value) = o {
    xs.forEach(() => collected.push(value.toUpperCase()));
  }
  return collected;
}

export {};


//// [ifLetBindingsStayNarrowedInsideClosures.ts]

type Opt =
  | { kind: "Some"; value: string }
  | { kind: "None" };
const Opt = {
  Some: (value: string): Opt => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
function f(o: Opt, xs: number[]): string[] {
  const collected: string[] = [];
  {
    const $tt_t0 = o;
    if ($tt_t0.kind === "Some") {
      const { value } = $tt_t0;
      xs.forEach(() => collected.push(value.toUpperCase()));
    }
  }
  return collected;
}

export {};
