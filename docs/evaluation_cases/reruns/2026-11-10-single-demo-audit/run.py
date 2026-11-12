import json,pathlib,subprocess,re
root=pathlib.Path('/Users/taku/workspace/kenkyu/proj-ref/refsyn')
out=pathlib.Path(__file__).resolve().parent
summary=[]
for name in ['insert','setAt','append','prepend','popBack','rotateRight']:
 original=json.loads((root/'tests/recordings'/name/'request.json').read_text())
 for selection in list(range(len(original['method_calls'])))+['all']:
  req=json.loads(json.dumps(original))
  if selection!='all':
   call=req['method_calls'][selection]
   req['method_calls']=[call]
   req['validation']['cases']=[c for c in req['validation']['cases'] if c['callLabel']==call['callLabel'] and c['contextSensitiveID']==call['contextSensitiveID']]
   assert len(req['validation']['cases'])==1
  d=out/f'{name}-{selection}';d.mkdir(exist_ok=True)
  (d/'request.json').write_text(json.dumps(req,indent=2))
  p=subprocess.run([str(root/'target/debug/examples/run_payload'),str(d/'request.json')],cwd=root,text=True,capture_output=True,timeout=100)
  (d/'run.log').write_text(p.stdout+'\nSTDERR\n'+p.stderr)
  m=re.search(r'HTTP (\d+)[^\n]*\n(\{.*)',p.stdout,re.S)
  if not m: raise RuntimeError(p.stdout+p.stderr)
  resp=json.loads(m[2]);(d/'response.json').write_text(json.dumps(resp,indent=2))
  row={'method':name,'selection':selection,'examples':len(req['method_calls']),'http':int(m[1]),'validation':resp.get('validation'),'holes':[(r['name'],r['success'],r.get('rendered')) for r in resp.get('escher_results') or []]}
  if resp.get('composed_method_code'):
   inp={'reference':str(root/'tests/recordings'/name/'reference.mjs'),'className':req['validation']['classes'][0]['name'],'method':req['method_calls'][0]['methodName'],'helpers':resp['code'],'composedMethod':resp['composed_method_code']}
   (d/'comparison-input.json').write_text(json.dumps(inp,indent=2))
   c=subprocess.run(['node','scripts/compare_with_reference.mjs'],cwd=root,input=json.dumps(inp),text=True,capture_output=True,timeout=45)
   (d/'comparison.json').write_text(c.stdout);(d/'comparison.stderr').write_text(c.stderr)
   row['comparison_exit']=c.returncode
   row['comparison']=json.loads(c.stdout) if c.stdout else {'error':c.stderr}
  summary.append(row)
  (out/'summary.json').write_text(json.dumps(summary,indent=2))
  comp=row.get('comparison',{})
  print(name,selection,'HTTP',m[1],row['validation'],'comparison',json.dumps(comp)[:190],flush=True)
