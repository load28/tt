//// [aDirectPipelineCallKeepsContextualTyping.tt] ////
const value: number = 1 |> (x => x + 1);

export {};


//// [aDirectPipelineCallKeepsContextualTyping.ts]
const value: number = (x => x + 1)(1);

export {};
