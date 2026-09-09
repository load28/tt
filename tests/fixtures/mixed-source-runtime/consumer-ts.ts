import { produceTs } from "./producer-ts.js";
import { produceTsx } from "./producer-tsx.jsx";
import { produceTt } from "./producer-tt.tt";
import { produceTtx } from "./producer-ttx.ttx";

export function consumeTs(): string[] {
  return [produceTs(), produceTsx(), produceTt(), produceTtx()].map(
    (value) => `ts<-${value}`,
  );
}
