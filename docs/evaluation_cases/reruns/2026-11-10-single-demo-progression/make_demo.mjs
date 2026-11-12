// Construct an explicit edit specification from a reference input; not a UI recording.
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
const [method,indexText,output]=process.argv.slice(2);
const spec=await import(pathToFileURL(`${process.cwd()}/tests/recordings/${method}/reference.mjs`).href);
const cls=method==='rotateRight'?'TreeNode':'Node';
const C=new Function(`${spec.classSource};return ${cls}`)();
const input=spec.cases[Number(indexText)].build({[cls]:C});
const ids=new Map(),queue=[];
const id=o=>{if(!ids.has(o)){ids.set(o,`ce${queue.length}`);queue.push(o);}return ids.get(o);};
id(input.receiver);for(const v of Object.values(input.roots))if(v&&typeof v==='object')id(v);
for(let k=0;k<queue.length;k++)for(const v of Object.values(queue[k]))if(v&&typeof v==='object')id(v);
const values={},objects=queue.map(o=>({id:id(o),className:cls,fields:Object.fromEntries(Object.entries(o).map(([f,v])=>{if(v&&typeof v==='object')return[f,{ref:id(v)}];values[id(o)+'-'+f]=v;return[f,v];}))}));
const operations=[];
const lit=(v)=>{const n='lit'+operations.length;operations.push({editType:'addNode',id:n,label:String(v),isLiteral:true,type:'number'});return n;};
const add=(from,label,to)=>operations.push({editType:'addEdge',from,label,to});
const edit=(o,f,to)=>{const old=o[f];if(old===null){add(id(o),f,to);}else operations.push({editType:'editEdgeReference',from:id(o),label:f,oldTo:typeof old==='object'?id(old):id(o)+'-'+f,newTo:to});};
const ret=to=>operations.push({editType:'addVariable',label:'return',to});
let prev=input.receiver;
if(method==='setAt'||method==='insert')for(let k=0;k<input.args[0];k++)prev=prev.next;
if(method==='setAt')edit(prev,'val',lit(input.args[1]));
if(['insert','append','prepend'].includes(method)){
 const n='created';operations.push({editType:'addNode',id:n,label:cls,isLiteral:false});
 add(n,'val',lit(input.args.at(-1)));
 if(method==='insert'){if(prev.next!==null)add(n,'next',id(prev.next));edit(prev,'next',n);}
 if(method==='append'){while(prev.next!==null)prev=prev.next;add(id(prev),'next',n);}
 if(method==='prepend'){add(n,'next',id(prev));ret(n);}
}
if(method==='popBack'){while(prev.next.next!==null)prev=prev.next;operations.push({editType:'deleteEdge',from:id(prev),label:'next',to:id(prev.next)});}
if(method==='rotateRight'){const l=prev.left;edit(prev,'left',id(l.right));edit(l,'right',id(prev));ret(id(l));}
const original=JSON.parse(fs.readFileSync(`tests/recordings/${method}/request.json`));
const call={...original.method_calls[0],callLabel:'counterexample'+indexText,contextSensitiveID:'independent',receiverObject:id(input.receiver),arguments:input.args,operations,idMapping:{},actualGraph:{nodes:[],edges:[]},precondGraph:{nodes:[],edges:[]}};
const validation={callLabel:call.callLabel,contextSensitiveID:call.contextSensitiveID,objects,values,arguments:input.args};
fs.writeFileSync(output,JSON.stringify({source:spec.cases[Number(indexText)].name,caseIndex:Number(indexText),call,validation},null,2));
