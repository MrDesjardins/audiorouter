import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import net from 'node:net';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import path from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { WebSocketServer } from 'ws';

const waitFor=async(check)=>{const deadline=Date.now()+6000;while(!check()){assert.ok(Date.now()<deadline,'fixture timed out');await new Promise(r=>setTimeout(r,20));}};
test('standalone service follows WS snapshots, calls HTTP and restores on shutdown', {timeout:15000},async()=>{
  const dir=await mkdtemp(path.join(tmpdir(),'audiorouter-stats-service-'));
  const changes=[];let child;
  const c=JSON.parse(await readFile(new URL('./config.example.json',import.meta.url),'utf8'));
  const graph={id:'fixture',nodes:[{id:'mix',name:c.mixer,kind:'mixer',enabled:true,bypass:false},
    {id:'eq',name:c.gameInput,enabled:true},{id:'discord',name:c.discordInput,enabled:true}],
    edges:[{sourceNode:'eq',destinationNode:'mix',enabled:true},{sourceNode:'discord',destinationNode:'mix',enabled:true}]};
  const api=http.createServer(async(req,res)=>{
    assert.equal(req.headers.authorization,'Bearer fixture-secret');
    let body='';for await(const chunk of req)body+=chunk;
    const result=req.url.endsWith('/active')?{sessionId:'fixture'}:req.url.endsWith('/get')?graph:{activation:{native:{state:'applied'}}};
    if(req.url.endsWith('/set'))changes.push(JSON.parse(body));
    res.writeHead(200,{'Content-Type':'application/json'});res.end(JSON.stringify(result));
  });
  await new Promise(r=>api.listen(0,'127.0.0.1',r));
  const ws=new WebSocketServer({port:0,host:'127.0.0.1'});await once(ws,'listening');
  const probe=net.createServer();await new Promise(r=>probe.listen(0,'127.0.0.1',r));const statusPort=probe.address().port;await new Promise(r=>probe.close(r));
  try {
    const config=path.join(dir,'config.json');
    await writeFile(config,JSON.stringify({...c,audioRouterUrl:`http://127.0.0.1:${api.address().port}`,statsUrl:`ws://127.0.0.1:${ws.address().port}`,statusPort}));
    const connection=once(ws,'connection');
    child=spawn(process.execPath,[fileURLToPath(new URL('./server.mjs',import.meta.url))],{env:{...process.env,AUDIOROUTER_API_TOKEN:'fixture-secret',AUDIOROUTER_INTEGRATION_CONFIG:config},stdio:['pipe','pipe','pipe'],windowsHide:true});
    let output='';child.stdout.on('data',b=>output+=b);child.stderr.on('data',b=>output+=b);
    const [socket]=await connection;
    await new Promise(r=>setTimeout(r,50));assert.equal(changes.length,0,'no invented initial match/menu state');
    const send=phase=>socket.send(JSON.stringify({status:'connected',match:phase==='menu'?null:{phase,ended_at:null},players:[{private:'should not persist'}]}));
    for(const [phase,percent]of [['menu',30],['prep',30],['action',100],['planning',30],['action',100],['results',30]]){
      send(phase);await waitFor(()=>changes.at(-1)?.parameters['inputVolume:eq']===percent);
    }
    const status=await(await fetch(`http://127.0.0.1:${statusPort}/status`)).json();
    assert.equal(status.phase,'results');assert.equal(status.desiredPercent,30);
    assert.ok(!JSON.stringify(status).includes('fixture-secret'));assert.ok(!output.includes('should not persist'));
    socket.close();await waitFor(()=>changes.at(-1)?.parameters['inputVolume:eq']===100);
    const finished=once(child,'exit');child.stdin.write('quit\n');
    const watchdog=setTimeout(()=>child.kill(),5000);
    try {assert.equal((await finished)[0],0);}finally{clearTimeout(watchdog);}
    assert.equal(changes.at(-1).parameters['inputVolume:eq'],100);
    for(const change of changes){assert.equal(change.parameters['inputVolume:discord'],100);assert.equal(change.node,'mix');assert.equal(change.sessionId,'fixture');}
  }finally{
    if(child&&child.exitCode===null)child.kill();
    for(const socket of ws.clients)socket.terminate();await new Promise(r=>ws.close(r));
    api.closeAllConnections();await new Promise(r=>api.close(r));await rm(dir,{recursive:true,force:true});
  }
});
