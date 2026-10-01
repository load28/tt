//// [matchOnBuiltinResultIsExhaustivenessChecked.tt] ////
const f = (r: Result<number, string>) => match (r) { Err(error) => error };

