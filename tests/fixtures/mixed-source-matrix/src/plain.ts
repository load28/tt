import type { TsxNode } from "./plain-jsx";
import type { TtNode } from "./language.tt";
import type { TtxNode } from "./language-jsx.ttx";

export interface TsNode {
  readonly source: "ts";
  readonly value: number;
}

export type TsEdges = readonly [TsxNode, TtNode, TtxNode];

export const tsNode = (value: number): TsNode => ({ source: "ts", value });
