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
        $tt_v0 = await import('./model.js');
        break;
      }
      case false: {
        $tt_v0 = await import('./fallback.js');
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + JSON.stringify($tt_m));
      }
    }
  }
  return $tt_v0;
}
