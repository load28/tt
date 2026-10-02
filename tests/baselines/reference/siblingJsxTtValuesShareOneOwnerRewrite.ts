//// [siblingJsxTtValuesShareOneOwnerRewrite.ttx] ////
declare const n: number;
const view = () => (<aside>
  {match (n) { 0 => <b>zero</b>, _ => <b>other</b> }}
  {match (n) { 0 => <i>zero</i>, _ => <i>other</i> }}
</aside>);


//// [siblingJsxTtValuesShareOneOwnerRewrite.tsx]
declare const n: number;
const view = () => {
  let $tt_subject;
  let $tt_subject_1;
  
  return (<aside>
  {($tt_subject = n, ($tt_subject === 0) ? <b>zero</b> : <b>other</b>)}
  {($tt_subject_1 = n, ($tt_subject_1 === 0) ? <i>zero</i> : <i>other</i>)}
</aside>);
};
