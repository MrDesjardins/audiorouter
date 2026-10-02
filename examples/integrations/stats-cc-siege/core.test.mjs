import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { AudioRouter, ApiError, LevelController, TargetError, localUrl, phaseState, resolveMixer, mixerRequest, validateConfig } from './core.mjs';
import { configureFeed, feedFile } from './setup-stats.mjs';

const c=JSON.parse(await readFile(new URL('./config.example.json',import.meta.url),'utf8'));
const graph=()=>({id:'fixture-session',nodes:[
  {id:'mix',name:c.mixer,kind:'mixer',enabled:true,bypass:false},
  {id:'eq',name:c.gameInput,kind:'parametricEq',enabled:true},
  {id:'discord',name:c.discordInput,kind:'applicationCapture',enabled:true}
],edges:[{sourceNode:'eq',destinationNode:'mix',enabled:true},{sourceNode:'discord',destinationNode:'mix',enabled:true}]});
const tick=()=>new Promise(r=>setTimeout(r,10));

test('only action is 100; prep and all known menu/selection phases are 30',()=>{
  const states=[
    [{status:'connected',match:null,startedQueuingAt:null},'menu',30],
    [{status:'connected',match:null,startedQueuingAt:1},'queue',30],
    ...[null,'planning','prep','action','results'].map(phase=>[{status:'connected',match:{phase,ended_at:null}},phase??'match-start',phase==='action'?100:30]),
    [{status:'connected',match:{phase:'action',ended_at:1}},'results',30],
    [{status:'connected',match:null},'menu',30]
  ];
  for(const [snapshot,phase,percent]of states)assert.deepEqual(phaseState(snapshot,c),{phase,percent});
});
test('unknown schema or unavailable data restores 100 rather than guessing a quiet phase',()=>{
  for(const snapshot of [null,{},[],{status:'loading'},{status:'error'}, {status:'disconnected'},
    {status:'connected'},{status:'connected',match:{}},{status:'connected',match:{phase:42}}])assert.equal(phaseState(snapshot,c).percent,100);
  assert.deepEqual(phaseState({status:'connected',match:{phase:'new-selection-phase'}},c),{phase:'other-phase',percent:30});
});
test('validates loopback addresses, config levels and port',()=>{
  assert.deepEqual(validateConfig(c),c);
  for(const url of ['http://example.com:123','http://localhost:123','http://127.0.0.1:123?token=x','http://a:b@127.0.0.1:123'])assert.throws(()=>localUrl(url,'http:'));
  assert.throws(()=>validateConfig({...c,quietPercent:Infinity}));
  assert.throws(()=>validateConfig({...c,statusPort:0}));
});
test('uses direct upstream IDs; changes no master, EQ, flags or routing',()=>{
  const target=resolveMixer(graph(),c);
  assert.deepEqual(mixerRequest(target,50,c,'event-1'),{sessionId:'fixture-session',node:'mix',parameters:{'inputVolume:eq':50,'inputVolume:discord':100},idempotencyKey:'event-1'});
});
test('rejects ambiguous, disconnected, disabled, bypassed and wrong-kind mixer targets',()=>{
  for(const mutate of [g=>g.nodes.push({...g.nodes[0],id:'other'}),g=>g.edges.pop(),g=>g.edges[0].enabled=false,
    g=>g.nodes[0].kind='volume',g=>g.nodes[0].bypass=true,g=>g.nodes[1].enabled=false]){
    const g=graph();mutate(g);assert.throws(()=>resolveMixer(g,c));
  }
});
test('API pins original active session and checks live activation',async()=>{
  const requests=[];
  const api=new AudioRouter(c,'fixture-token',async(url,options)=>{
    requests.push({url,body:options.body?JSON.parse(options.body):null});
    if(url.endsWith('/sessions/active'))return Response.json({sessionId:'fixture-session'});
    if(url.endsWith('/sessions/get'))return Response.json(graph());
    return Response.json({activation:{native:{state:'applied'}}});
  });
  await api.discover();await api.set(50,'event-1');await api.set(100,'event-2');
  assert.equal(requests.filter(r=>r.url.endsWith('/sessions/active')).length,1);
  assert.deepEqual(requests.at(-1).body.parameters,{'inputVolume:eq':100,'inputVolume:discord':100});
  api.fetcher=async(url)=>Response.json(url.endsWith('/sessions/get')?graph():{activation:{native:{state:'restartRequired'}}});
  await assert.rejects(()=>api.set(50,'event-3'),TargetError);
});
test('does not print private API error body or token',async()=>{
  const api=new AudioRouter(c,'super-secret',async()=>Response.json({error:{message:'private-player-and-token'}},{status:401}));
  await assert.rejects(()=>api.discover(),e=>e instanceof ApiError&&!e.message.includes('private-player')&&!e.message.includes('super-secret'));
});
test('serializes and coalesces transitions; retries one event key',async()=>{
  const writes=[];let release;let fail=true;
  const controller=new LevelController(async(p,key)=>{writes.push({p,key});if(writes.length===1)await new Promise(r=>release=r);if(fail){fail=false;throw new Error('temporary');}},()=>{},1);
  controller.desire(50);controller.desire(100);controller.desire(50);release();
  for(let n=0;n<20&&controller.running;n++)await tick();
  assert.equal(writes.at(-1).p,50);assert.equal(controller.applied,50);
  const retry=new LevelController(async(p,key)=>{writes.push({p,key});if(writes.length===3)throw new Error('once');},()=>{},1);
  retry.desire(100);for(let n=0;n<20&&retry.running;n++)await tick();
  assert.equal(writes.at(-1).key,writes.at(-2).key);
  await retry.stop(100);assert.equal(writes.at(-1).p,100);
});
test('target/auth failures stop retries; shutdown still attempts explicit restore',async()=>{
  let calls=0;
  const controller=new LevelController(async()=>{calls++;throw new ApiError(401,'denied');},()=>{},1);
  controller.desire(50);await tick();controller.desire(100);await tick();assert.equal(calls,1);
  await assert.rejects(()=>controller.stop(100),ApiError);assert.equal(calls,2);
});
test('setup is repeatable, refuses another port and removes only matching file',async()=>{
  const dir=await mkdtemp(path.join(tmpdir(),'audiorouter-stats-fixture-'));
  try {
    const net=await import('node:net');const probe=net.createServer();
    await new Promise(r=>probe.listen(0,'127.0.0.1',r));const port=probe.address().port;await new Promise(r=>probe.close(r));
    assert.equal(await configureFeed(dir,port),'configured');
    assert.equal((await readFile(path.join(dir,feedFile),'utf8')).trim(),String(port));
    assert.equal(await configureFeed(dir,port),'already configured');
    await assert.rejects(()=>configureFeed(dir,port===17894?17895:17894));
    assert.equal(await configureFeed(dir,port,true),'removed');
  }finally{await rm(dir,{recursive:true,force:true});}
});
