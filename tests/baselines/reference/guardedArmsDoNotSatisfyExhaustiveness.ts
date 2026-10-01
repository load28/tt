//// [guardedArmsDoNotSatisfyExhaustiveness.tt] ////
const f = (o: Option<number>) => match (o) { Some(value) if value > 0 => value, None => 0 };

