import pathlib,json,subprocess
root=pathlib.Path.cwd();out=pathlib.Path(__file__).resolve().parent;old=root/'docs/evaluation_cases/reruns/2026-11-10-single-demo-audit'
summary=[]
for item in json.loads((out/'refinement-summary.json').read_text()):
 method=item['method'];start=item['start'];accepted=[h for h in item['history'] if h['validation']['status']=='passed']
 source=out/accepted[-1]['dir'] if accepted else old/f'{method}-{start}'
 response=json.loads((source/'response.json').read_text());d=out/f'holdout-{method}-{start}';d.mkdir(exist_ok=True)
 ref=d/'reference.mjs';ref.write_text('export {classSource} from '+json.dumps((root/'tests/recordings'/method/'reference.mjs').as_uri())+';\nimport {listCases,treeCases} from "../holdout.mjs";\nexport const cases = '+('treeCases()' if method=='rotateRight' else 'listCases('+json.dumps(method)+')')+';\n')
 inp={'reference':str(ref),'className':'TreeNode' if method=='rotateRight' else 'Node','method':method,'helpers':response['code'],'composedMethod':response['composed_method_code']}
 (d/'comparison-input.json').write_text(json.dumps(inp,indent=2))
 p=subprocess.run(['node','scripts/compare_with_reference.mjs'],input=json.dumps(inp),capture_output=True,text=True,timeout=90);assert p.returncode==0,p.stderr
 c=json.loads(p.stdout);(d/'comparison.json').write_text(p.stdout)
 summary.append({'method':method,'start':start,'examples':item['final_examples'],'cases':c['cases'],'matches':c['cases']-len(c['mismatches'])})
 print(summary[-1],flush=True)
(out/'holdout-summary.json').write_text(json.dumps(summary,indent=2))

assert all(x["cases"] > 0 and x["matches"] == x["cases"] for x in summary), "held-out comparison mismatch"
