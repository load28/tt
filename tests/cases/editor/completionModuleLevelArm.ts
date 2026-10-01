type Outcome = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function f(n: number): number;
declare function read(n: number): Outcome;
declare function run<T>(body: () => T): T;
declare function unwrap(o: Outcome): number;
declare const input: number;
export const top = ((t: number) => { if (t === 1) return f(/*arm*/input); if (t === 2) { const b = f(/*block*/input); return b; } return input as /*typed*/number; })(input);
export const scrutinee = ((t: number) => 0)(/*scrutinee*/input);
export const outcome = run((): Outcome => { const k = unwrap(read(/*inResult*/input)); return { kind: "Ok", value: k }; });
