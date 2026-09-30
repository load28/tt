//// [tsconfig.json] ////
{
  "compilerOptions": {
    "target": "es2022",
    "module": "nodenext",
    "moduleResolution": "nodenext",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  }
}

//// [package.json] ////
{ "name": "case", "private": true, "type": "module" }

//// [shapes.tt] ////
export const shapeK = 1;

//// [main.tt] ////
import { shapeK } from "./shapes.tt.js";
import { shapeK as again } from "./shapes.tt";
export const y = shapeK + again;
export const later = import("./shapes.tt.js");


//// [main.ts]
import { shapeK } from "./shapes.tt.js";
import { shapeK as again } from "./shapes.js";
export const y = shapeK + again;
export const later = import("./shapes.tt.js");

//// [shapes.ts]
export const shapeK = 1;
