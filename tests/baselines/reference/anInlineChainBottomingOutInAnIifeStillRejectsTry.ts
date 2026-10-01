//// [anInlineChainBottomingOutInAnIifeStillRejectsTry.tt] ////
variant E { A(x: number), B }
const r = match (e) {
  A(x) => { if let A(y) = f(x) { const n = try g(y); return n; } return 0; },
  B => 0,
};

