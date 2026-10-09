//// [aSuperPipelineHeadThatIsNotAReceiverIsNotTypeScript.tt] ////
declare class B { value: number }
declare function f(x: unknown): number;
export class C extends B { m() { return super |> f; } }

