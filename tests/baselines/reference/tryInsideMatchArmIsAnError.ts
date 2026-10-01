//// [tryInsideMatchArmIsAnError.tt] ////
const x = match (r) {
  Ok(value) => { const y = try f(value); return y; },
  Err(error) => fallback(error),
};

