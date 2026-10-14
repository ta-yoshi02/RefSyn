import fs from 'node:fs';
import { pathToFileURL } from 'node:url';
const [taskPath, resultPath, adapterPath] = process.argv.slice(2);
const {runRefsynTasks} = await import(pathToFileURL(adapterPath));
const task=JSON.parse(fs.readFileSync(taskPath,'utf8'));
const result=runRefsynTasks([task],{maxCost:5,timeoutMs:5000,quiet:true});
const method=result[0].compiled_js;
const actual=[-2,2].map(val=>{
  const obj={val};
  if(method) Object.assign(obj,eval(`({${method}})`));
  return method?obj.negativeHas():null;
});
const output={taskInput:task.examples,result,actualJavaScriptResult:actual,expectedTaskResult:[false,true]};
fs.writeFileSync(resultPath,JSON.stringify(output,null,2));
console.log(JSON.stringify(output,null,2));
