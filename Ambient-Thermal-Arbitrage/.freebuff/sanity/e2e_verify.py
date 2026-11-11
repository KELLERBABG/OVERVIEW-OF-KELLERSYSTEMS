import base64, hashlib, hmac, json, os, pathlib, shutil, socket, subprocess, time, urllib.error, urllib.request
root=pathlib.Path(__file__).resolve().parents[2]
work=root/'.freebuff/sanity/e2e-work';work.mkdir(exist_ok=True)
for port in [1502,18086]:
    s=socket.socket();s.bind(('127.0.0.1',port));s.close()
shutil.copy2(root/'scripts/e2e/plc-config.toml',work/'config.toml')
(work/'data_cache').mkdir(exist_ok=True)
for p in (root/'crates/gateway/data_cache').glob('*.json'):shutil.copy2(p,work/'data_cache'/p.name)
plog=open(work/'plc.log','w',encoding='utf-8');glog=open(work/'gateway.log','w',encoding='utf-8')
plc=gw=None;checks=[]
def request(path,data=None,token=None):
    headers={'Content-Type':'application/json'}
    if token:headers['Authorization']='Bearer '+token
    req=urllib.request.Request('http://127.0.0.1:18086'+path,data=data,headers=headers)
    try:
        with urllib.request.urlopen(req,timeout=2) as r:return r.status,r.read().decode()
    except urllib.error.HTTPError as e:return e.code,e.read().decode()
def wait(predicate,seconds=30):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        try:
            result=predicate()
            if result:return result
        except (OSError,ValueError):pass
        time.sleep(.25)
    raise AssertionError('condition timed out')
try:
    plc=subprocess.Popen(['node',str(root/'scripts/e2e/fake-plc.js'),'1502'],stdin=subprocess.DEVNULL,stdout=plog,stderr=subprocess.STDOUT)
    env=os.environ.copy();env.update(ATA_SECRET='sanity-test-only',RUST_LOG='info')
    gw=subprocess.Popen([str(root/'target/debug/thermal-arbitrage-mvp.exe')],cwd=work,env=env,stdin=subprocess.DEVNULL,stdout=glog,stderr=subprocess.STDOUT)
    def status():
        code,body=request('/status');return json.loads(body) if code==200 else {}
    def emergency_status():
        s=status()
        return s if s.get('current_temp_c')==20.0 and s.get('active_controller')=='Emergency' and s.get('compressor_on') is True else None
    result=wait(emergency_status)
    assert result['grid_frequency_hz']==49.98
    checks.append('PLC temperature, Emergency, ON and actual frequency verified through /status')
    wait(lambda:'COIL addr=1 ON' in (work/'plc.log').read_text(encoding='utf-8'))
    checks.append('actual fake-PLC FC05 command receipt verified (not real plant feedback)')
    token=base64.urlsafe_b64encode(hmac.new(b'sanity-test-only',b'/price',hashlib.sha256).digest()).decode().rstrip('=')
    assert request('/price',b'{"price_eur_per_mwh":80}')[0]==401
    assert request('/price',b'{"price_eur_per_mwh":NaN}',token)[0]==400
    assert request('/price',b'{"price_eur_per_mwh":80}',token)[0]==200
    checks.append('HTTP 401/400/200 explicitly verified')
    plc.terminate();plc.wait(timeout=5);plc=None
    wait(lambda:'FAIL-SAFE: forced cooling' in (work/'gateway.log').read_text(encoding='utf-8'))
    assert 'Sensor/comm loss' in (work/'gateway.log').read_text(encoding='utf-8')
    assert request('/health')[0]==200
    assert status()['compressor_on'] is True
    checks.append('PLC loss: fail-safe log, sensor-loss log, live health, command ON verified; no physical cooling proof')
    print(json.dumps({'passed':checks,'status':result},ensure_ascii=False,indent=2))
finally:
    for p in [gw,plc]:
        if p is not None and p.poll() is None:p.terminate();p.wait(timeout=5)
    plog.close();glog.close()
    print('Owned subprocesses stopped and awaited')
