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
export type LazyView = typeof import('./view.jsx');
export const load = () => import(/* retained */ './model.js');
export const withOptions = () => import('./data.js', { with: { type: 'json' } });
export const computed = (suffix: string) => import('./model.tt' + suffix);
export const view = <button onClick={() => import('./view.jsx')}>Load</button>;
export async function selected(flag: boolean) {
  let $tt_v0;
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        const $tt_a0 = { value: await import('./model.js') }; $tt_v0 = $tt_a0.value;
        break;
      }
      case false: {
        const $tt_a1 = { value: await import('./fallback.js') }; $tt_v0 = $tt_a1.value;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}
