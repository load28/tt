//// [optionalPostfixUsesTheCommonReceiverGroupingRule.tt] ////
const a = value |> ?.member;
const b = left + right |> ?.member;
const c = make() |> ?.member;


//// [optionalPostfixUsesTheCommonReceiverGroupingRule.ts]
const a = value?.member;
const b = (left + right)?.member;
const c = make()?.member;
