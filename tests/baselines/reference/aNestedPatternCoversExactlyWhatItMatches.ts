//// [aNestedPatternCoversExactlyWhatItMatches.tt] ////

const n = match (r) {
  Ok(value: Some(value: v)) => v,
  Err(error) => 0,
};

