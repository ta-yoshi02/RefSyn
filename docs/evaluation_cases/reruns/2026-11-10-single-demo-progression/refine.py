import pathlib,json,subprocess,re
root=pathlib.Path.cwd();out=pathlib.Path(__file__).resolve().parent
old=root/'docs/evaluation_cases/reruns/2026-11-10-single-demo-audit'
summary=[]
def run(req,d,method):
 d.mkdir(exist_ok=True);(d/'request.json').write_text(json.dumps(req,indent=2))
 p=subprocess.run([str(root/'target/debug/examples/run_payload'),str(d/'request.json')],capture_output=True,text=True,timeout=100)
 (d/'run.log').write_text(p.stdout+'\n'+p.stderr)
 m=re.search(r'HTTP (\d+)[^\n]*\n(\{.*)',p.stdout,re.S);assert m,p.stderr
 r=json.loads(m[2]);(d/'response.json').write_text(json.dumps(r,indent=2))
 row={'examples':len(req['method_calls']),'http':int(m[1]),'validation':r.get('validation'),'dir':d.name}
 if r.get('composed_method_code'):
  inp={'reference':str(root/'tests/recordings'/method/'reference.mjs'),'className':req['validation']['classes'][0]['name'],'method':method,'helpers':r['code'],'composedMethod':r['composed_method_code']}
  p=subprocess.run(['node','scripts/compare_with_reference.mjs'],input=json.dumps(inp),capture_output=True,text=True,timeout=45)
  assert p.returncode==0,p.stderr
  c=json.loads(p.stdout);(d/'comparison.json').write_text(p.stdout);row.update(c)
 return row
for method,start in [('insert','0'),('setAt','0'),('append','0'),('prepend','0'),('popBack','0'),('rotateRight','0'),('rotateRight','all')]:
 req=json.loads((old/f'{method}-{start}'/'request.json').read_text());c=json.loads((old/f'{method}-{start}'/'comparison.json').read_text())
 names=json.loads(subprocess.check_output(['node','--input-type=module','-e',f"import {{cases}} from './tests/recordings/{method}/reference.mjs'; console.log(JSON.stringify(cases.map(c=>c.name)));"],text=True))
 history=[];used=set();attempt=0
 while c['mismatches']:
  candidates=[names.index(x['case']) for x in c['mismatches'] if names.index(x['case']) not in used]
  if not candidates:break
  index=candidates[0];used.add(index);attempt+=1
  d=out/f'refine-{method}-{start}-{attempt}';d.mkdir(exist_ok=True)
  subprocess.run(['node',str(out/'make_demo.mjs'),method,str(index),str(d/'added-demo.json')],check=True)
  demo=json.loads((d/'added-demo.json').read_text());trial=json.loads(json.dumps(req));trial['method_calls'].append(demo['call']);trial['validation']['cases'].append(demo['validation'])
  row=run(trial,d,method);row['selected_counterexample']=demo['source'];history.append(row)
  if row['validation'] and row['validation']['status']=='passed':req=trial;c={'cases':row['cases'],'mismatches':row['mismatches']}
  print(method,start,'attempt',attempt,'examples',row['examples'],'validation',row['validation'],'matches',row.get('cases',0)-len(row.get('mismatches',[])),'/',row.get('cases',0),flush=True)
 summary.append({'method':method,'start':start,'history':history,'final_examples':len(req['method_calls']),'final_matches':c['cases']-len(c['mismatches']),'cases':c['cases']})
 (out/'refinement-summary.json').write_text(json.dumps(summary,indent=2))
 (out/f'final-{method}-{start}.json').write_text(json.dumps(req,indent=2))

assert all(x["cases"] > 0 and x["final_matches"] == x["cases"] for x in summary), "counterexamples remain"
