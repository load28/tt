//// [duplicateBindingWithinOnePatternIsAnError.tt] ////
const n = match (r) { Ok(value: Some(value), error: value) => value, _ => 0 };

