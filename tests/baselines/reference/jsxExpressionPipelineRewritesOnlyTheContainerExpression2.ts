//// [jsxExpressionPipelineRewritesOnlyTheContainerExpression2.ttx] ////
declare const raw: string; declare const up: (x: string) => string;
const child = <P value={raw |> up} />;


//// [jsxExpressionPipelineRewritesOnlyTheContainerExpression2.tsx]
declare const raw: string; declare const up: (x: string) => string;
const child = <P value={(($tt_v, $tt_f) => $tt_f($tt_v))(raw, up)} />;
