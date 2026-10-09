//// [aTryInASetterIsAPlacementError.tt] ////
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
declare function r(n: number): R<number>;
export class C {
    v = 0;
    set x(n: number) { const a = try r(n); this.v = a; }
    set y(n: number) { this.v = n + (try r(n)); }
    m(n: number): R<number> { const a = try r(n); return { kind: "Ok", value: a }; }
}
export const o = {
    v: 0,
    set w(n: number) { const a = try r(n); this.v = a; },
    set z(n: number) { this.v = n + (try r(n)); },
};

