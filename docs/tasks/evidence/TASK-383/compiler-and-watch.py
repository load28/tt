from pathlib import Path
import subprocess,json,time
root=Path.cwd();base=root/'target/audit-383';ttc=str(root/'target/debug/ttc');base.mkdir(parents=True,exist_ok=True);results=[]
cases={
 'two-match-calls':'const value = Number(match (1) { 1 => 1, _ => 0 }) + Number(match (2) { 2 => 2, _ => 0 });\n',
 'flow-match':'const value = [(flow |> ((x: number) => x + 1))(1), match (1) { 1 => 1, _ => 0 }];\n',
 'ttx-pipe-match':'declare const h: any; const view = <main a={(1 |> ((x: number) => x + 1))}>{match (1) { 1 => 1, _ => 0 }}</main>;\n'
}
for name,source in cases.items():
 p=base/(name+('.ttx' if name.startswith('ttx') else '.tt'));p.write_text(source)
 r=subprocess.run([ttc,'-p','--no-banner',str(p)],text=True,capture_output=True,timeout=30);results.append({'case':name,'source':source,'exit':r.returncode,'stderr':r.stderr,'stdout':r.stdout})
for mode in ['inplace-watch','types-watch']:
 p=base/mode;p.mkdir(exist_ok=True);(p/'src').mkdir(exist_ok=True);(p/'src/a.tt').write_text('export const value = 1;\n');(p/'tsconfig.json').write_text('{"compilerOptions":{"strict":true,"module":"preserve","moduleResolution":"bundler"},"include":["src"]}')
 args=['--watch','src'] if mode=='inplace-watch' else ['--types','--watch',str(p/'src'),'-o','types']
 with (p/'log.txt').open('w') as log:
  proc=subprocess.Popen([ttc,*args],cwd=p,stdout=log,stderr=log,text=True)
  try:
   end=time.time()+20
   while 'Ctrl-C' not in (p/'log.txt').read_text() and time.time()<end:time.sleep(.1)
   time.sleep(1)
   if mode=='inplace-watch':(p/'src/a.tt').write_text('export const value = 2;\n')
   else:(p/'src/b.tt').write_text('export const newValue = 2;\n')
   time.sleep(2)
  finally:proc.terminate();proc.wait(timeout=5)
 results.append({'case':mode,'log':(p/'log.txt').read_text(),'files':{str(f.relative_to(p)):f.read_text() for f in p.rglob('*.ts')}})
(base/'more.json').write_text(json.dumps(results,indent=2));print(json.dumps(results,indent=2))
