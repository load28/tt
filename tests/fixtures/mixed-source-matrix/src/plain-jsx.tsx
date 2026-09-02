import type { SameTsx } from "./same-jsx";
import type { TsNode } from "./plain";
import type { TtNode } from "./language.tt";
import type { TtxNode } from "./language-jsx.ttx";

declare global {
  namespace JSX {
    interface IntrinsicElements {
      span: Record<string, unknown>;
      section: Record<string, unknown>;
      strong: Record<string, unknown>;
      code: Record<string, unknown>;
      aside: Record<string, unknown>;
    }
  }
}

export interface TsxNode {
  readonly source: "tsx";
  readonly label: string;
}

export type TsxEdges = readonly [SameTsx, TsNode, TtNode, TtxNode];

export const TsxBadge = ({ node }: { readonly node: TsxNode }) => (
  <span data-source={node.source}>{node.label}</span>
);
