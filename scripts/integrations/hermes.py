"""Hermes observer: no tool arguments, message text, or permission decisions."""
import hashlib, json, os, time
from pathlib import Path

def emit(session_id, state):
    if not isinstance(session_id,str) or not session_id or len(session_id)>512: return
    try:
        identity=hashlib.sha256(session_id.encode()).hexdigest()
        root=Path(os.environ.get('LOCALAPPDATA') or Path.home())/'Verge'/'signals'
        root.mkdir(parents=True,exist_ok=True)
        dest=root/f'agent-hermes-{identity}.json'
        temp=dest.with_suffix(f'.{os.getpid()}.tmp')
        temp.write_text(json.dumps(dict(provider='hermes',session=identity,state=state,at=int(time.time()*1000))),encoding='utf-8')
        os.replace(temp,dest)
    except OSError: pass

def start(session_id=None,*args,**kwargs): emit(session_id,'working')
def end(session_id=None,completed=False,interrupted=False,*args,**kwargs): emit(session_id,'completed' if completed and not interrupted else 'stopped')
def waiting(session_key=None,surface=None,**kwargs):
    if surface != 'smart': emit(session_key,'waiting')
def resumed(session_key=None,choice=None,**kwargs): emit(session_key,'stopped' if choice in ['deny','timeout','notify_failed','smart_deny'] else 'working')
def register(ctx):
    ctx.register_hook('pre_llm_call',start)
    ctx.register_hook('on_session_end',end)
    ctx.register_hook('pre_approval_request',waiting)
    ctx.register_hook('post_approval_response',resumed)
