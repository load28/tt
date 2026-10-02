//// [letElseInsideMatchArmIsError.tt] ////
const x = match (r) {
  Ok(value) => { const Some(v) = h(value) else { return 0; }; return v; },
  _ => 0,
};

