//// [nestedPatternInsideOrPatternIsAnError.tt] ////
const n = match (r) { Ok(value: Some(v)) | Err(error) => 1, _ => 0 };

