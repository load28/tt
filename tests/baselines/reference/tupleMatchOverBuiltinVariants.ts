//// [tupleMatchOverBuiltinVariants.tt] ////
const r = match (o, r2) {
  (Some(value), Ok(value: v)) => value + v,
  (None, _) => 0,
};

