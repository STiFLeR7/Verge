import importlib.util,json,os,tempfile,sys
sys.dont_write_bytecode=True
from pathlib import Path
spec=importlib.util.spec_from_file_location('observer',Path(__file__).with_name('hermes.py'))
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
with tempfile.TemporaryDirectory() as directory:
    os.environ['LOCALAPPDATA']=directory
    def state(): return json.loads(next((Path(directory)/'Verge/signals').glob('*.json')).read_text())
    module.start(session_id='s',user_message='private')
    assert state()['state']=='working'
    module.waiting(session_key='s',surface='cli',command='private')
    assert state()['state']=='waiting'
    module.resumed(session_key='s',choice='deny')
    assert state()['state']=='stopped'
    module.end(session_id='s',completed=True,interrupted=False)
    assert state()['state']=='completed'
    assert set(state())=={'provider','session','state','at'}
print('PASS: Hermes event mapping; metadata only; no decisions returned.')
