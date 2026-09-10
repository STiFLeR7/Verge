import {observe} from './verge-observer-lib.mjs';
export default function(pi) {
  let failed=false;
  const emit=(ctx,state)=>observe('pi',ctx.sessionManager.getSessionId(),state);
  pi.on('agent_start',async(_event,ctx)=>{failed=false;emit(ctx,'working');});
  pi.on('message_update',async(_event,ctx)=>{emit(ctx,'working');});
  pi.on('message_end',async(event,ctx)=>{
    if (event.message?.role==='assistant' && ['error','aborted'].includes(event.message.stopReason)) {failed=true;emit(ctx,'stopped');}
  });
  pi.on('agent_settled',async(_event,ctx)=>{emit(ctx,failed?'stopped':'completed');});
  pi.on('ui_prompt_start',async(_event,ctx)=>{emit(ctx,'waiting');});
  pi.on('ui_prompt_end',async(_event,ctx)=>{emit(ctx,ctx.isIdle()?'idle':'working');});
}
