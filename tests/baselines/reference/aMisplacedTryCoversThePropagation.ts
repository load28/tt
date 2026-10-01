//// [aMisplacedTryCoversThePropagation.tt] ////
const x = match (r) {
  Ok(v) => { const y = try f(v); return y; },
  Err(e) => 0,
};

