declare namespace JSX { interface IntrinsicElements { div: { title?: string }; span: {}; } }
declare const flag: boolean;
const view = <div title={ }>{(flag ? 1 : 2)}</div>;
const a = (flag ? 1 : 2);
const b = (flag ? 3 : 4);
const later = "정상";
const wrong: number = "wrong";
later./*member*/toUpperCase();
/*later*/later;
