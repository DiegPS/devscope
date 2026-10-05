"""Repeatable release scan benchmark; all fixtures live under ignored target/."""
import json, os, statistics, subprocess, sys
from pathlib import Path
root=Path(__file__).resolve().parent.parent
label=sys.argv[1] if len(sys.argv)>1 else 'baseline'
env={**os.environ,'DS_BENCH_ROOT':str(root/'target/bench-fixture')}
result=subprocess.run(['cargo','test','--locked','--release','--bin','ds','benchmark_scan_fixture','--','--ignored','--exact','regression_tests::benchmark_scan_fixture','--nocapture','--test-threads=1'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,check=True)
line=next(line for line in result.stdout.splitlines() if 'DS_BENCH_JSON=' in line)
rows=json.loads(line.split('DS_BENCH_JSON=',1)[1])
summary={}
for key in ['scan_ms','full_scan_ms']:
    values=sorted(row[key] for row in rows)
    summary[key]={'median':statistics.median(values),'p95':values[18],'min':values[0],'max':values[-1]}
out=root/'target/bench-reports';out.mkdir(parents=True,exist_ok=True)
(out/(label+'.json')).write_text(json.dumps({'label':label,'projects':1000,'git_repos':200,'source_files':16000,'warmups':3,'samples':rows,'summary':summary},indent=2),encoding='utf-8')
print(json.dumps({'label':label,'summary':summary}),flush=True)
