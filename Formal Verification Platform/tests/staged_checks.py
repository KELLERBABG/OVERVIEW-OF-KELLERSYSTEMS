#!/usr/bin/env python3
"""Staged architecture CLI acceptance checks (real Cargo, rustc and Z3)."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
VERIFIER = ROOT / 'target/debug' / ('keller-verify.exe' if os.name == 'nt' else 'keller-verify')


def command(cwd, *args, env=None):
    started = time.perf_counter()
    p = subprocess.run([str(VERIFIER), 'keller', 'verify', '--json', *args], cwd=cwd,
                       env=env, capture_output=True, encoding='utf-8', timeout=120)
    assert p.returncode in (0, 1), (p.returncode, p.stdout, p.stderr)
    summary = json.loads(p.stdout)
    run = Path(summary['run_directory'])
    record = json.loads((run / 'report.run.json').read_text())
    assert record['completed'] and record['run_directory'] == str(run)
    assert record['finding_evidence'] or summary['findings'] == []
    assert all(e['native_reproduction'] == 'not-attempted' for e in record['finding_evidence'])
    return p, summary, record, time.perf_counter() - started


with tempfile.TemporaryDirectory(prefix='keller-staged-') as tmp:
    cwd = Path(tmp)
    (cwd / 'src').mkdir()
    (cwd / 'rust-toolchain.toml').write_text((ROOT / 'rust-toolchain.toml').read_text())
    (cwd / 'Cargo.toml').write_text('[package]\nname="staged_probe"\nversion="0.0.0"\nedition="2021"\n[workspace]\n')
    source = cwd / 'src/main.rs'
    source.write_text('pub fn shift(x:u8)->u8 { (x << 1) + 255 }\n'
                      'pub fn cast(x:u16)->u8 { (x as u8) + 255 }\n'
                      'pub fn masked(x:u8)->u8 { (x & 0) + 255 }\n'
                      'fn main() {}\n')
    p, summary, record, _ = command(cwd)
    failed = {e['function'] for e in summary['findings'] if e['status'] == 'FAIL'}
    assert {'shift', 'cast'} <= failed, summary
    assert 'masked' not in failed, summary
    print('[PASS] staged: symbolic shift/cast failures remain visible while masked arithmetic is proved')

    source.write_text('#![allow(unconditional_panic)]\nfn main() { let x:u8=128; let y=x << 1; let _z=1u8/y; }\n')
    p, summary, _, _ = command(cwd)
    assert any(e['kind'] == 'divzero' for e in summary['findings']), summary
    print('[PASS] staged: constant truncated shift cannot become false PASS on downstream zero division')

    source.write_text('pub fn large(x:u128)->u128 { x + 340282366920938463463374607431768211455u128 }\nfn main() {}\n')
    p, summary, _, _ = command(cwd)
    assert any(w['value'] == '340282366920938463463374607431768211455'
               for e in summary['findings'] for w in e.get('witness', [])), summary
    print('[PASS] staged: unsigned-128 literal evidence is exact, never clamped')

    source.write_text('pub fn wrapping(x:u8)->u8 { let y=x+1; 1/y }\nfn main() {}\n')
    env = os.environ.copy()
    env.pop('CARGO_ENCODED_RUSTFLAGS', None)
    env['RUSTFLAGS'] = '-C overflow-checks=off'
    p, summary, _, _ = command(cwd, env=env)
    assert any(e['kind'] == 'divzero' for e in summary['findings']), summary
    print('[PASS] staged: unchecked-profile wrapping arithmetic retains reachable downstream failures')

    argv = [str(VERIFIER), 'keller', 'verify', '--json']
    procs = [subprocess.Popen(argv, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                             encoding='utf-8', env=env) for _ in range(2)]
    runs = []
    for proc in procs:
        out, err = proc.communicate(timeout=120)
        assert proc.returncode == 1, (out, err)
        runs.append(Path(json.loads(out)['run_directory']))
    assert runs[0] != runs[1]
    for run in runs:
        assert list((run / '.keller-reports').glob('*.started'))
        assert json.loads((run / 'report.run.json').read_text())['completed']
        assert json.loads((run / 'report.json').read_text())
    root_record = json.loads((cwd / 'report.run.json').read_text())
    root_run = Path(root_record['run_directory'])
    assert root_run in runs
    assert (cwd / 'report.json').read_bytes() == (root_run / 'report.json').read_bytes()
    assert (cwd / 'report.environment.json').read_bytes() == (root_run / 'report.environment.json').read_bytes()
    print('[PASS] staged: simultaneous invocations retain distinct complete evidence and publish a consistent compatibility view')

    dep = cwd / 'dependency'
    (dep / 'src').mkdir(parents=True)
    (dep / 'Cargo.toml').write_text('[package]\nname="cache_dependency"\nversion="0.0.0"\nedition="2021"\n')
    (dep / 'src/lib.rs').write_text('\n'.join(f'pub fn f{i}(x:u64)->u64 {{ x.wrapping_mul({i+1}) }}' for i in range(1500)))
    (cwd / 'Cargo.toml').write_text('[package]\nname="staged_probe"\nversion="0.0.0"\nedition="2021"\n'
                                  '[dependencies]\ncache_dependency={path="dependency"}\n'
                                  '[workspace]\nexclude=["dependency"]\n')
    source.write_text('pub fn add(x:u8)->u8 { x+1 }\nfn main() { println!("{}",cache_dependency::f1(3)); }\n')
    measurement = []
    reports = []
    for label, args in [('cold', ['--no-cache']), ('cache_fill', []), ('warm', [])]:
        p, summary, record, elapsed = command(cwd, *args)
        assert summary['coverage']['functions'] > 0 and summary['counts']['fail'] > 0
        assert summary['coverage']['workers'] == 1, 'dependency must not be instrumented'
        built = 'Checking cache_dependency' in p.stderr
        if label == 'warm':
            assert not built, p.stderr
        else:
            assert built, p.stderr
        reports.append((Path(summary['run_directory']) / 'report.json').read_bytes())
        measurement.append({'mode': label, 'seconds': round(elapsed, 3), 'dependency_compiled': built,
                            'workspace_workers': summary['coverage']['workers']})
    assert reports[0] == reports[1] == reports[2]
    print('[PASS] staged: cold/fill/warm verdicts agree; warm dependencies are cached and workspace analysis still runs')
    destination = ROOT / 'target/staged-measurement.json'
    destination.write_text(json.dumps({'input': 'one workspace package + 1500-function local dependency',
                                       'measurements': measurement}, indent=2))
    print('MEASUREMENT ' + json.dumps(measurement))
