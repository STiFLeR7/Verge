import {observe} from './verge-observer-lib.mjs';
export default {id:'verge-observer',server:async()=>{
  const failed=new Set();
  return {event:async({event})=>{
    const p=event.properties || {}, id=p.sessionID || p.sessionId;
    let state;
    if(event.type==='session.status') {
      if(['busy','retry'].includes(p.status?.type)) {failed.delete(id);state='working';}
      if(p.status?.type==='idle') state=failed.has(id)?'stopped':'completed';
    }
    if(event.type==='session.idle') state=failed.has(id)?'stopped':'completed';
    if(event.type==='session.error') {failed.add(id);state='stopped';}
    if(event.type==='permission.asked') state='waiting';
    if(event.type==='permission.replied') state='working';
    if(state) observe('kilo',id,state);
  }};
}};
