import { consumeTs } from "./consumer-ts.js";
import { consumeTsx } from "./consumer-tsx.jsx";
import { consumeTt } from "./consumer-tt.tt";
import { consumeTtx } from "./consumer-ttx.ttx";
import { trace } from "./runtime-state.js";

console.log(JSON.stringify({
  values: [
    ...consumeTs(),
    ...consumeTsx(),
    ...consumeTt(),
    ...consumeTtx(),
  ],
  trace,
}));
