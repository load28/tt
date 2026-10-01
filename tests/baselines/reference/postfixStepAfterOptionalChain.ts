//// [main.tt] ////
export {};
function box(present: boolean): { inner: { value: number } } | undefined {
  return present ? { inner: { value: 1.25 } } : undefined;
}
function loose(present: boolean): any {
  return present ? " a " : undefined;
}
function attempt(label: string, run: () => unknown): void {
  try {
    console.log(label, JSON.stringify(run()));
  } catch (error) {
    console.log(label, "threw", error instanceof TypeError);
  }
}
function fixed(present: boolean): string {
  return box(present)?.inner.value! |> .toFixed(1);
}
for (const present of [true, false]) {
  attempt("head", () => fixed(present));
  attempt("member", () => box(present)?.inner! |> .value |> String);
  attempt("tail", () => loose(present) |> ?.trim() |> .length);
}

//// [twin.ts] ////
export {};
function box(present: boolean): { inner: { value: number } } | undefined {
  return present ? { inner: { value: 1.25 } } : undefined;
}
function loose(present: boolean): any {
  return present ? " a " : undefined;
}
function attempt(label: string, run: () => unknown): void {
  try {
    console.log(label, JSON.stringify(run()));
  } catch (error) {
    console.log(label, "threw", error instanceof TypeError);
  }
}
function fixed(present: boolean): string {
  return (box(present)?.inner.value!).toFixed(1);
}
for (const present of [true, false]) {
  attempt("head", () => fixed(present));
  attempt("member", () => String((box(present)?.inner!).value));
  attempt("tail", () => (loose(present)?.trim()).length);
}


//// [main.ts]
export {};
function box(present: boolean): { inner: { value: number } } | undefined {
  return present ? { inner: { value: 1.25 } } : undefined;
}
function loose(present: boolean): any {
  return present ? " a " : undefined;
}
function attempt(label: string, run: () => unknown): void {
  try {
    console.log(label, JSON.stringify(run()));
  } catch (error) {
    console.log(label, "threw", error instanceof TypeError);
  }
}
function fixed(present: boolean): string {
  return (box(present)?.inner.value!).toFixed(1);
}
for (const present of [true, false]) {
  attempt("head", () => fixed(present));
  attempt("member", () => (($tt_v, $tt_f) => $tt_f($tt_v))((box(present)?.inner!).value, String));
  attempt("tail", () => (loose(present)?.trim()).length);
}
\ No newline at end of file

//// [twin.ts]
export {};
function box(present: boolean): { inner: { value: number } } | undefined {
  return present ? { inner: { value: 1.25 } } : undefined;
}
function loose(present: boolean): any {
  return present ? " a " : undefined;
}
function attempt(label: string, run: () => unknown): void {
  try {
    console.log(label, JSON.stringify(run()));
  } catch (error) {
    console.log(label, "threw", error instanceof TypeError);
  }
}
function fixed(present: boolean): string {
  return (box(present)?.inner.value!).toFixed(1);
}
for (const present of [true, false]) {
  attempt("head", () => fixed(present));
  attempt("member", () => String((box(present)?.inner!).value));
  attempt("tail", () => (loose(present)?.trim()).length);
}
