function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}
import type { Model } from './model.js';
export type LazyView = typeof import('./view.js');
export const load = () => import(/* retained */ './model.js');
export const withOptions = () => import('./data.js', { with: { type: 'json' } });
export const computed = (suffix: string) => import('./model.tt' + suffix);
export const view = <button onClick={() => import('./view.js')}>Load</button>;
export async function selected(flag: boolean) {
  let $tt_v0;
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        $tt_v0 = await import('./model.js');
        break;
      }
      case false: {
        $tt_v0 = await import('./fallback.js');
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}
