//// [pipelineTypeErrorInAStepIsReportedOnUserText.tt] ////
const n: number = 1 |> ((a: string) => a.length);

export {};


//// [pipelineTypeErrorInAStepIsReportedOnUserText.ts]
const n: number = ((a: string) => a.length)(1);

export {};
