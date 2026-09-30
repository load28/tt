declare global { namespace JSX { interface IntrinsicElements { button: { label?: string; onClick?: () => void } } } }
/** A labelled button. */
export function Button(props: { /*prop*/label: string }) {
  return <button label={props./*member*/label} onClick={() => /*arrow*/console.log(props.label)} />;
}
export const view = <Button /*attribute*/label="ok" />;
const unused = <button label={1} />;
