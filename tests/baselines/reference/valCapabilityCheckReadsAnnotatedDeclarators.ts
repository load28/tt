//// [valCapabilityCheckReadsAnnotatedDeclarators.tt] ////
type Handler = (u: Box) => void;
const update: Handler = (u) => { u.n = 1; };
function f(val b: Box) {
  update(b);
}

