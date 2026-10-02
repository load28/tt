//// [tsconfig.json] ////
{
  "compilerOptions": {
    "target": "es2022",
    "module": "esnext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "jsxFactory": "id1 id2"
  }
}

//// [a.tt] ////
export const x: number = "s";


//// [a.ts]
export const x: number = "s";
