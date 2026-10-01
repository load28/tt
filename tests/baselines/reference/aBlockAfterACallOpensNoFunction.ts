//// [aBlockAfterACallOpensNoFunction.tt] ////
declare function f(): Option<number>;
declare function g(): Result<number, string>;
if let Some(v) = f() { try g(); }

