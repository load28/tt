//// [aPipelineStepThatIsNotAPrimaryExpressionIsCalledAsAGroup.tt] ////
declare const f: ((n: number) => string) | undefined;
declare const g: (n: number) => string;
declare const as: (n: number) => string;
const a = 1 |> f ?? g;
const b = 1 |> await Promise.resolve(g);
const c = 1 |> as;
export {};


//// [aPipelineStepThatIsNotAPrimaryExpressionIsCalledAsAGroup.ts]
declare const f: ((n: number) => string) | undefined;
declare const g: (n: number) => string;
declare const as: (n: number) => string;
const a = (f ?? g)(1);
const b = (await Promise.resolve(g))(1);
const c = as(1);
export {};
