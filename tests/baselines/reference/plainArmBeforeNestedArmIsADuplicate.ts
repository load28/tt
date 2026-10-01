//// [plainArmBeforeNestedArmIsADuplicate.tt] ////
const n = match (r) { Ok(value) => 1, Ok(value: Some(value: v)) => v, _ => 0 };

