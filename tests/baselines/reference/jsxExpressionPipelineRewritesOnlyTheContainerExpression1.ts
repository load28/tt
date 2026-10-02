//// [jsxExpressionPipelineRewritesOnlyTheContainerExpression1.ttx] ////
declare const raw: string; declare const up: (x: string) => string;
const child = <p>{raw |> up}</p>;


//// [jsxExpressionPipelineRewritesOnlyTheContainerExpression1.tsx]
declare const raw: string; declare const up: (x: string) => string;
const child = <p>{(($tt_v, $tt_f) => $tt_f($tt_v))(raw, up)}</p>;
