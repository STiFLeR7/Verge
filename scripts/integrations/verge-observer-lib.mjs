// Metadata-only observer: never emits a permission decision or stores prompts/tool arguments.
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
let previous = new Map();
export function observe(provider, session, state) {
  if (!['kilo','pi','hermes'].includes(provider) || !['working','completed','stopped','waiting','idle'].includes(state)) return;
  if (typeof session !== 'string' || !session || session.length > 512) return;
  const id=createHash('sha256').update(session).digest('hex');
  const key=provider+id, now=Date.now(), last=previous.get(key);
  if (last?.state===state && now-last.at<2000) return;
  try {
    const root=path.join(process.env.LOCALAPPDATA || process.env.HOME,'Verge','signals');
    fs.mkdirSync(root,{recursive:true});
    const dest=path.join(root,`agent-${provider}-${id}.json`), temp=dest+'.'+process.pid+'.tmp';
    fs.writeFileSync(temp,JSON.stringify({provider,session:id,state,at:now}));
    fs.renameSync(temp,dest);
    previous.set(key,{state,at:now});
  } catch {} // An optional observer cannot break the host tool.
}
