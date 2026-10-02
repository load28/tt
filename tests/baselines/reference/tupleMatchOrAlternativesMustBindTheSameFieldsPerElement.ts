//// [tupleMatchOrAlternativesMustBindTheSameFieldsPerElement.tt] ////
const r = match (a, b) {
  (Some(value) | None, _) => 1,
  _ => 0,
};

