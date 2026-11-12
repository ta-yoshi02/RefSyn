// Fixed finite check inputs, not used to choose additional demonstrations.
export function listCases(method) {
 const cases=[];
 for(const length of [1,2,3,4,6,9,16]) for(const pattern of [0,1,2,3]) {
  if(method==='popBack'&&length<2)continue;
  const positions=['insert','setAt'].includes(method)?Array.from({length},(_,i)=>i):[0];
  for(const i of positions)cases.push({name:`len=${length},pattern=${pattern},i=${i}`,build({Node}){
   const vals=Array.from({length},(_,k)=>pattern===0?0:pattern===1?7:pattern===2?100-k*31:(k%2?-(k+1)*13:(k+1)*17));
   const nodes=vals.map(v=>{const n=new Node();n.val=v;return n;});
   nodes.forEach((n,k)=>n.next=nodes[k+1]??null);
   const value=[-101,0,7,113][pattern];const args=['insert','setAt'].includes(method)?[i,value]:['append','prepend'].includes(method)?[value]:[];
   const detached=new Node();detached.val=-99;
   const parent=new Node();parent.val=200;parent.next=nodes[0];
   return {receiver:nodes[0],args,roots:{head:nodes[0],middle:nodes[Math.floor(length/2)],tail:nodes.at(-1),parent,detached}};
  }});
 }
 return cases;
}
export function treeCases() {
 let state=731;const rand=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state;};
 const shape=(depth)=>depth===0||rand()%4===0?null:[(rand()%31)-15,shape(depth-1),shape(depth-1)];
 const shapes=Array.from({length:160},(_,i)=>[i-80,[i%9,shape(1+i%4),i%5===0?null:[i%7,shape(i%3),shape(i%4)]],shape(i%5)]);
 return shapes.map((s,i)=>({name:`seed731-tree${i},middleNull=${i%5===0}`,build({TreeNode}){
  const make=t=>{if(t===null)return null;const n=new TreeNode();[n.val]=t;n.left=make(t[1]);n.right=make(t[2]);return n;};
  const root=make(s);const parent=new TreeNode();parent.val=999;parent.left=root;
  return {receiver:root,args:[],roots:{parent,root,pivot:root.left,middle:root.left.right}};
 }}));
}
