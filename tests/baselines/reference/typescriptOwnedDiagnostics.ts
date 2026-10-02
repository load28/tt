//// [typescriptOwnedDiagnostics.tt] ////
// A checker diagnostic whose span TypeScript places on text the user wrote is
// TypeScript's: its code, message, and range. One whose span lands on code
// ttc generated is restated over the construct.
declare function total(): number;
declare function takes(s: string): void;
const inc = (n: number): number => n + 1;
export const annotated: string = total();
takes(total());
export function returned(): string {
  return total();
}
takes(1 |> inc);


//// [typescriptOwnedDiagnostics.ts]
// A checker diagnostic whose span TypeScript places on text the user wrote is
// TypeScript's: its code, message, and range. One whose span lands on code
// ttc generated is restated over the construct.
declare function total(): number;
declare function takes(s: string): void;
const inc = (n: number): number => n + 1;
export const annotated: string = total();
takes(total());
export function returned(): string {
  return total();
}
takes(inc(1));
