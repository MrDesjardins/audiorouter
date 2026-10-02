import { readFile } from 'node:fs/promises';
import http from 'node:http';
import { createInterface } from 'node:readline';
import { Writable } from 'node:stream';
import WebSocket from 'ws';
import { AudioRouter, LevelController, localUrl, phaseState, validateConfig } from './core.mjs';
import { savedToken } from './saved-token.mjs';

const flags = new Set(process.argv.slice(2));
if ([...flags].some(f=>!['--inspect','--observe','--dry-run'].includes(f))) throw new Error('Options: --inspect, --observe, --dry-run.');
const defaults=JSON.parse(await readFile(new URL('./config.example.json',import.meta.url),'utf8'));
let override={};
try { override=JSON.parse(await readFile(process.env.AUDIOROUTER_INTEGRATION_CONFIG ?? new URL('./config.local.json',import.meta.url),'utf8')); } catch(e) { if(e.code!=='ENOENT')throw e; }
const c=validateConfig({...defaults,...override});

async function tokenPrompt() {
  if (process.env.AUDIOROUTER_API_TOKEN) { const token=process.env.AUDIOROUTER_API_TOKEN; delete process.env.AUDIOROUTER_API_TOKEN; return token; }
  const saved=await savedToken();if(saved)return saved;
  if (!process.stdin.isTTY) throw new Error('Set AUDIOROUTER_API_TOKEN privately or run from an interactive terminal.');
  process.stdout.write('AudioRouter API token (hidden): ');
  const muted=new Writable({write(_chunk,_encoding,callback){callback();}});
  const rl=createInterface({input:process.stdin,output:muted,terminal:true});
  return new Promise(resolve=>rl.question('',value=>{rl.close();process.stdout.write('\n');resolve(value.trim());}));
}

const observe=flags.has('--observe');
const dryRun=flags.has('--dry-run');
let api=null;
if (!observe) {
  const token=await tokenPrompt(); if (!token || /[\r\n]/.test(token)) throw new Error('Invalid API token.');
  api=new AudioRouter(c,token); const target=await api.discover();
  console.log(`Pinned session ${target.sessionId}; Mixer ${target.mixerId}; Siege input ${target.gameId}; Discord input ${target.discordId}.`);
  if(flags.has('--inspect')) { console.log('Read-only inspection complete. No levels changed.'); process.exit(0); }
}

const status={source:'waiting',phase:'unknown',desiredPercent:null,appliedPercent:null,discordPercent:c.discordPercent,mode:observe?'observe':dryRun?'dry-run':'live',api:'idle'};
const log=(event,fields={})=>console.log(JSON.stringify({time:new Date().toISOString(),event,...fields}));
const controller=new LevelController(async(percent,key)=>{
  if(!observe&&!dryRun) await api.set(percent,key);
},(event,percent,error)=>{
  status.api=event;
  if(event==='applied') { if(!observe&&!dryRun)status.appliedPercent=percent; log(dryRun||observe?'would-set':'level-set',{siege:percent,discord:c.discordPercent}); }
  // Do not print arbitrary vendor/HTTP payloads, tokens or player information.
  else log('api-error',{status:error?.status??null,guidance:'Check API, token and graph. Transient failures retry; authorization, target or activation errors pause writes until restart.'});
});

const server=http.createServer((req,res)=>{
  if(req.method!=='GET'||req.url!=='/status'||req.headers.host!==`127.0.0.1:${c.statusPort}`) {res.writeHead(404);res.end();return;}
  res.writeHead(200,{'Content-Type':'application/json','Cache-Control':'no-store','Content-Security-Policy':"default-src 'none'",'X-Content-Type-Options':'nosniff'});
  res.end(JSON.stringify(status));
});
server.requestTimeout=3000;server.headersTimeout=3000;server.maxConnections=8;
await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(c.statusPort,'127.0.0.1',resolve);});
console.log(`Status: http://127.0.0.1:${c.statusPort}/status`);
let socket=null,retryTimer=null,heartbeat=null,stopping=false,backoff=1000;
const commands=createInterface({input:process.stdin,terminal:false});
commands.on('line',line=>{if(line.trim().toLowerCase()==='quit')void stop();});
console.log('Type quit and press Enter, or press Ctrl+C, to restore levels and stop.');
function unavailable() {
  status.source='disconnected';status.phase='unavailable';status.desiredPercent=c.restorePercent;
  controller.desire(c.restorePercent);
}
function connect() {
  if(stopping)return;
  socket=new WebSocket(localUrl(c.statsUrl,'ws:'),{maxPayload:1024*1024,handshakeTimeout:3000,perMessageDeflate:false});
  socket.on('open',()=>{
    backoff=1000; status.source='waiting-for-snapshot'; log('stats-connected');
    // A live but quiet match sends no snapshots. Stats.cc's pings prove liveness.
    let alive=true;
    socket.on('ping',()=>{alive=true;}); socket.on('pong',()=>{alive=true;});
    heartbeat=setInterval(()=>{if(!alive){socket.terminate();return;}alive=false;socket.ping();},35000);
  });
  socket.on('message',(data,binary)=>{
    if(binary)return;
    let state; try {state=phaseState(JSON.parse(data.toString()),c);}catch{state={phase:'unknown',percent:c.restorePercent};}
    status.source='connected';
    if(status.phase!==state.phase)log('phase',{phase:state.phase,siege:state.percent,discord:c.discordPercent});
    status.phase=state.phase;status.desiredPercent=state.percent;controller.desire(state.percent);
  });
  socket.on('error',()=>{}); // close schedules bounded retry; no raw WS error output.
  socket.on('close',()=>{
    if(heartbeat)clearInterval(heartbeat);heartbeat=null;
    if(stopping)return;
    unavailable();log('stats-unavailable',{retryMs:backoff});
    retryTimer=setTimeout(connect,backoff);backoff=Math.min(backoff*2,15000);
  });
}
async function stop() {
  if(stopping)return;stopping=true;commands.close();process.stdin.pause();process.stdin.unref?.();clearTimeout(retryTimer);clearInterval(heartbeat);socket?.terminate();server.close();server.closeAllConnections();
  try {await controller.stop(c.restorePercent);log('stopped',{restoreSiege:c.restorePercent,discord:c.discordPercent});}
  catch {log('restore-failed',{guidance:'Set both Mixer input levels to 100% manually in AudioRouter.'});process.exitCode=1;}
  // Node's Windows stdin/HTTP handles can stay referenced after close. All
  // restoration and cleanup has finished; allow final stdout to drain first.
  process.stdout.write('',()=>process.exit(process.exitCode??0));
}
process.once('SIGINT',()=>void stop());process.once('SIGTERM',()=>void stop());
connect();
