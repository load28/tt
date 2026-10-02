// @filename: tsconfig.json
{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  }
}
// @filename: other.ts
export function mkOther() { return 1; }
// @filename: shapes.ts
export const shapeK = 1;
// @filename: main.ts
import { mkOther } from "./other.ts";
export const y = mkOther() + shapeK/*withTsEnding*/;
