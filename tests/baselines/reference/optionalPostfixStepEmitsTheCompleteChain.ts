//// [optionalPostfixStepEmitsTheCompleteChain.tt] ////
const a = x |> ?.trim();
const b = xs |> ?.[key]?.value;
const c = fn |> ?.(arg).value?.();


//// [optionalPostfixStepEmitsTheCompleteChain.ts]
const a = x?.trim();
const b = xs?.[key]?.value;
const c = fn?.(arg).value?.();
