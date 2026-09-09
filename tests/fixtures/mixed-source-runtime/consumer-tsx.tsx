import { produceTs } from "./producer-ts.js";
import { produceTsx } from "./producer-tsx.jsx";
import { produceTt } from "./producer-tt.tt";
import { produceTtx } from "./producer-ttx.ttx";

declare global {
  namespace JSX {
    interface IntrinsicElements {
      matrix: { values: string[] };
    }
  }
}

const h = (_tag: unknown, props: { values: string[] }): string[] => props.values;

export function consumeTsx(): string[] {
  return <matrix values={[
    produceTs(),
    produceTsx(),
    produceTt(),
    produceTtx(),
  ].map((value) => `tsx<-${value}`)} />;
}
