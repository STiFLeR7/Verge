// Local metadata only. Hooks emit no decisions and never grant permissions.
const fs = require('node:fs');
const path = require('node:path');
const root = () => path.join(process.env.LOCALAPPDATA || process.env.HOME, 'Verge', 'signals');
function write(name, value) {
  const dir = root(); fs.mkdirSync(dir, { recursive: true });
  const dest = path.join(dir, name + '.json');
  const tmp = dest + '.' + process.pid + '.tmp';
  try { fs.writeFileSync(tmp, JSON.stringify(value)); fs.renameSync(tmp, dest); }
  finally { try { fs.unlinkSync(tmp); } catch {} }
}
function usage(data) {
  try {
    intelligence(data);
    const windows = {};
    for (const key of ['five_hour', 'seven_day']) {
      const w = data.rate_limits?.[key];
      if (Number.isFinite(w?.used_percentage) && w.used_percentage >= 0 && w.used_percentage <= 100
          && Number.isSafeInteger(w.resets_at) && w.resets_at > 0) {
        windows[key] = { used_percentage: w.used_percentage, resets_at: w.resets_at };
      }
    }
    // An absent rate_limits field is not a reported zero.
    if (Object.keys(windows).length) write('claude-usage', { at: Date.now(), rate_limits: windows });
  } catch {} // The optional observer must not break the user's terminal status line.
}
function intelligence(data) {
  const id = data.session_id;
  if (typeof id !== 'string' || !/^[a-zA-Z0-9-]{1,100}$/.test(id)) return;
  const clean = value => typeof value === 'string'
    ? value.replace(/[\x00-\x1f\x7f\u202a-\u202e\u2066-\u2069]/g, '').slice(0, 160) : '';
  const modelId = clean(data.model?.id);
  const record = { session_id: id, at: Date.now(), model: modelId
    ? { id: modelId, name: clean(data.model?.display_name) || modelId } : null };
  const ctx = data.context_window;
  if (ctx !== undefined) {
    record.context = null;
    if (Number.isFinite(ctx?.used_percentage) && ctx.used_percentage >= 0 && ctx.used_percentage <= 100) {
      record.context = { used_percentage: ctx.used_percentage,
        capacity: Number.isSafeInteger(ctx.context_window_size) && ctx.context_window_size > 0
          ? ctx.context_window_size : null };
    }
  }
  write('claude-context-' + id, record);
}
function activity(data) {
  const id = data.session_id;
  if (typeof id !== 'string' || !/^[a-zA-Z0-9-]{1,100}$/.test(id)) return;
  const event = data.hook_event_name;
  const state = ({ UserPromptSubmit: 'busy', PreToolUse: 'busy', PostToolUse: 'busy',
    PostToolUseFailure: 'stopped', StopFailure: 'stopped', PermissionRequest: 'waiting', Stop: 'completed', SessionEnd: 'stopped' })[event]
    || (event === 'Notification' && data.notification_type === 'permission_prompt' ? 'waiting' : null);
  if (!state) return;
  const tool = typeof data.tool_name === 'string' ? data.tool_name.replace(/[^\w .:-]/g, '').slice(0, 80) : '';
  write('claude-session-' + id, { at: Date.now(), state, waiting_for: state === 'waiting'
    ? (tool ? 'Approve ' + tool + ' in Claude' : 'Approval needed in Claude') : null });
}
module.exports = { usage, activity, intelligence };
if (require.main === module) {
  let input = ''; process.stdin.setEncoding('utf8');
  process.stdin.on('data', chunk => { input += chunk; if (input.length > 1048576) process.exit(0); });
  process.stdin.on('end', () => { try { activity(JSON.parse(input)); } catch {} });
}
