declare global { namespace JSX { interface IntrinsicElements { table: {}; tr: { key?: string }; td: { colSpan?: number }; } } }
declare function money(n: number): string;
declare const lines: { sku: string; qty: number; price: number }[];
export function Cart() {
  const total = lines.length;
  return (
    <table>
      {lines.map((line) => (
        <tr key={line.sku}>
          <td>{line.qty * line.price + money.length</td>
        </tr>
      ))}
      <tr>
        <td colSpan={2}>{total === 0 ? "none" : /*total*/total}</td>
      </tr>
    </table>
  );
}
