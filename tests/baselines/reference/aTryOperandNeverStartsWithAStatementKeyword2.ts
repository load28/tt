//// [aTryOperandNeverStartsWithAStatementKeyword2.tt] ////
declare function f(): any;
function g() { const x = if (f()) {}; }

