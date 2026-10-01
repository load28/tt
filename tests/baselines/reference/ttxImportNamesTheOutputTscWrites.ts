//// [tsconfig.json] ////
{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  }
}

//// [view.ttx] ////
export const label: string = "view";
export const lazy = () => import("./lazy.ttx");

//// [lazy.ttx] ////
export const later: string = "lazy";

//// [main.tt] ////
import { label, lazy } from "./view.ttx";
console.log(label, (await lazy()).later);


//// [lazy.tsx]
export const later: string = "lazy";
\ No newline at end of file

//// [main.ts]
import { label, lazy } from "./view.js";
console.log(label, (await lazy()).later);

//// [view.tsx]
export const label: string = "view";
export const lazy = () => import("./lazy.js");
\ No newline at end of file
