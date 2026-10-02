//// [aTryOperandNeverStartsWithAStatementKeyword1.tt] ////
declare function f(): any;
function g() { const x = try if (f()) {}; }

