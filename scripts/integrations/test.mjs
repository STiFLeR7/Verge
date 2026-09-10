import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const dir=fs.mkdtempSync(path.join(os.tmpdir(),'verge-observers-'));
process.env.LOCALAPPDATA=dir;
try {
  fs.copyFileSync(new URL('./verge-observer-lib.mjs',import.meta.url),path.join(dir,'verge-observer-lib.mjs'));
  for(const name of ['pi','kilo']) {
    const source = new URL('./'+name+'.ts',import.meta.url);
    const relative = fs.readFileSync(source,'utf8').match(/from '([^']+)'/)[1];
    assert.ok(fs.existsSync(new URL(relative,source)), 'Repository plugin import must resolve without renaming its dependency');
    fs.copyFileSync(source,path.join(dir,name+'.mjs'));
  }
  const records=()=>fs.readdirSync(path.join(dir,'Verge/signals')).filter(n=>n.endsWith('.json')).map(n=>JSON.parse(fs.readFileSync(path.join(dir,'Verge/signals',n))));
  const kilo=(await import(pathToFileURL(path.join(dir,'kilo.mjs')))).default;
  const hooks=await kilo.server();
  await hooks.event({event:{type:'session.error',properties:{sessionID:'private-id',secret:'must not persist'}}});
  assert.equal(records()[0].state,'stopped');
  await hooks.event({event:{type:'session.idle',properties:{sessionID:'private-id'}}});
  assert.equal(records()[0].state,'stopped');
  await hooks.event({event:{type:'permission.asked',properties:{sessionID:'private-id'}}});
  assert.equal(records()[0].state,'waiting');
  const handlers={}, pi=(await import(pathToFileURL(path.join(dir,'pi.mjs')))).default;
  pi({on:(name,handler)=>{handlers[name]=handler;}});
  const ctx={sessionManager:{getSessionId:()=> 'pi-session'},isIdle:()=>true};
  await handlers.agent_start({},ctx);
  assert.equal(records().find(r=>r.provider==='pi').state,'working');
  await handlers.message_end({message:{role:'assistant',stopReason:'error',content:'private'}},ctx);
  await handlers.agent_settled({},ctx);
  assert.equal(records().find(r=>r.provider==='pi').state,'stopped');
  for(const r of records()) assert.deepEqual(Object.keys(r).sort(),['at','provider','session','state']);
  console.log('PASS: Pi/Kilo event mapping, completion/error distinction, no prompt/secret/approval persistence.');
} finally {fs.rmSync(dir,{recursive:true,force:true});}
