from pathlib import Path
import shutil

source = Path('external/escher-ts')
target = Path('.evaluation-weighted')
target.mkdir(exist_ok=True)
shutil.copytree(source / 'dist', target / 'dist', dirs_exist_ok=True)
shutil.copy2(source / 'package.json', target / 'package.json')
p = target / 'dist/synthesis/ascendrec/synthesizer.js'
s = p.read_text()
assert s.count('const compCost = 1;') == 1
p.write_text(s.replace('const compCost = 1;', 'const compCost = globalThis.__refsynCompCost?.[impl.name] ?? 1;'))
