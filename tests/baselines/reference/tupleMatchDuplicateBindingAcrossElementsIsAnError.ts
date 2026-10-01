//// [tupleMatchDuplicateBindingAcrossElementsIsAnError.tt] ////
const r = match (a, b) {
  (Some(value), Some(value)) => value,
  _ => 0,
};

