//// [runtimeAVarLetElseAsAnUnbracedBodyStaysUnderItsParent.tt] ////

variant O { Some(value: number), None }
function h(c: boolean, o: O): number | undefined {
  const read = () => hv;
  if (c) var Some(value: hv) = o else { return 0; };
  return read();
}
function w(xs: O[]): number {
  const read = () => wv;
  let i = 0;
  while (i < xs.length) var Some(value: wv) = xs[i++] else { break; };
  return read() ?? -1;
}
function e(c: boolean, o: O): number | undefined {
  if (c) return -3; else var Some(value: ev) = o else { return 0; };
  return ev;
}
function l(o: O): number {
  lbl: var Some(value: lv) = o else { return -2; };
  return lv + 1;
}
console.log(h(true, O.Some(5)), h(true, O.None), h(false, O.Some(5)));
console.log(w([O.Some(1), O.Some(2), O.None, O.Some(9)]), w([]));
console.log(e(true, O.None), e(false, O.Some(4)), e(false, O.None));
console.log(l(O.Some(1)), l(O.None));

export {};


//// [runtimeAVarLetElseAsAnUnbracedBodyStaysUnderItsParent.ts]

type O =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const O = {
  Some: (value: number): O => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
function h(c: boolean, o: O): number | undefined {
  const read = () => hv;
  if (c) {
    const $tt_t0 = o;
    if ($tt_t0.kind !== "Some") {
      return 0;
    }
    var { value: hv } = $tt_t0;
  }
  return read();
}
function w(xs: O[]): number {
  const read = () => wv;
  let i = 0;
  while (i < xs.length) {
    const $tt_t1 = xs[i++];
    if ($tt_t1.kind !== "Some") {
      break;
    }
    var { value: wv } = $tt_t1;
  }
  return read() ?? -1;
}
function e(c: boolean, o: O): number | undefined {
  if (c) return -3; else {
    const $tt_t2 = o;
    if ($tt_t2.kind !== "Some") {
      return 0;
    }
    var { value: ev } = $tt_t2;
  }
  return ev;
}
function l(o: O): number {
  lbl: {
    const $tt_t3 = o;
    if ($tt_t3.kind !== "Some") {
      return -2;
    }
    var { value: lv } = $tt_t3;
  }
  return lv + 1;
}
console.log(h(true, O.Some(5)), h(true, O.None), h(false, O.Some(5)));
console.log(w([O.Some(1), O.Some(2), O.None, O.Some(9)]), w([]));
console.log(e(true, O.None), e(false, O.Some(4)), e(false, O.None));
console.log(l(O.Some(1)), l(O.None));

export {};
