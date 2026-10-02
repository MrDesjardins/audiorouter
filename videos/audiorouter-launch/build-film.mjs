import fs from 'node:fs';
import {execFileSync} from 'node:child_process';
const scenes=[
 [0,4,'AudioRouter','YOUR SOUND.','YOUR WAY.','Your sound. Your setup. Your way.','Live tools. Visual control.',[['eq',4]]],
 [4,7,'Build it by sight','MOVE THROUGH','YOUR ROUTE.','Move around your mix. See every stage. Make it yours.','Canvas view · live playback · per-tool timing',[['route-drag',7]]],
 [11,10,'Visual tone shaping','SHAPE THE','SOUND.','Shape your tone. Choose a starting point. Make it yours.','Advanced EQ · Graphic EQ · Bass & Treble',[['eq',4],['graphic',3],['bass',3]]],
 [21,9,'Live dynamics','VOICE UP.','PEAKS DOWN.','Set your voice level in a tap. See the compressor follow a real voice sample as it plays.','Voice Compressor - live response',[['compressor',3],['gate',3],['limiter',3]]],
 [30,8,'Make room for your voice','CLEAN VOICE.','ROOM TO SPEAK.','Tame the room hum. Shape denoising. Let your game step back when you speak.','Speech Denoise - Hum Removal - Duck',[['denoise',2],['dehum',3],['duck',3]]],
 [38,6,'Create your workflow','YOUR PLUGINS.','YOUR WORKFLOW.','Bring your favorite plugin into the route, then move through a canvas built around your sound.','VST plugin - visual routing',[['plugin',3],['route-drag',3]]],
 [44,5,'Built for streamers','TWO PCS.','ONE STREAM.','Gaming PC to streaming PC. Send your mix across your home network.','Network Send → Network Receive',[['send',5],['receive',5]]],
 [49,5,'Know your audio journey','TUNE LIVE.','SEE THE TIMING.','Tune it live. See the timing of every tool, and your whole audio journey.','Per-tool processing. Your complete route.',[['timing',5]]],
 [54,12,'Control it your way','YOUR CONTROLS.','YOUR ASSISTANT.','Make it part of your workflow. Connect your controls through the API. Or let an AI assistant help configure your sound with MCP.','API + MCP',[['api',6],['mcp',6]]],
 [66,4,'Your sound. Your setup.','AudioRouter','See your sound. Shape it. Send it anywhere.','See your sound. Shape it. Send it anywhere. AudioRouter.','github.com/MrDesjardins/audiorouter',[]]
];
const names={eq:'Advanced EQ',graphic:'Graphic EQ',bass:'Bass & Treble',compressor:'Compressor',gate:'Voice Compressor controls',limiter:'Live gain reduction',spectral:'Spectral Gate',denoise:'Denoise',dehum:'Hum Removal',duck:'Duck',plugin:'ReaEQ · real plugin editor',recorder:'Recording · choose your format',send:'Network Send · Gaming PC',receive:'Network Receive · Streaming PC',timing:'Real processing readings',api:'API · connect your controls',mcp:'MCP · real successful tool call','route-drag':'Move your tools','arrange':'Arrange your signal flow'};
// Curated source ranges from the two fresh 1920x1080 AudioRouter-only OBS takes.
// Ranges keep the operator's real cursor gestures and the app's live visual response.
const captures={
  take1:'capture/reshoot/audiorouter-actual-tools-2026-10-02.mp4',
  take2:'capture/reshoot/audiorouter-live-properties-2026-10-02.mp4'
};
const ranges={
  eq:['take1',150,4], 'route-drag':['take1',47,7], graphic:['take2',68,3], bass:['take2',101,3],
  compressor:['take2',288,3], gate:['take2',291,3], limiter:['take2',294,3], dehum:['take2',420,3],
  denoise:['take1',0,2], duck:['take2',138,3], plugin:['take1',25,3], recorder:['take1',300,3],
  send:['take1',412,5], receive:['take2',0,5], timing:['take1',225,5], api:['take1',257,6],
  mcp:['take1',290,6], arrange:['take1',73,4]
};
// Rebuild each named clip directly from the captured UI. Never pad a short take with a freeze frame.
for(const [,, ,,,, ,shots] of scenes)for(const [name,duration] of shots){
 const output=`assets/clips/${name}-${duration}s.mp4`;
 const [take,offset,length]=ranges[name]??ranges.eq;
 if(length<duration)throw new Error(`No full moving take for ${name} (${duration}s)`);
 const vf=name==='mcp'?'crop=960:400:960:540,fps=30':'crop=1680:945:120:135,scale=1920:1080,fps=30';
 execFileSync('ffmpeg',['-y','-hide_banner','-loglevel','error','-ss',String(offset),'-i',captures[take],'-t',String(duration),'-an','-vf',vf,'-c:v','libx264','-crf','18','-pix_fmt','yuv420p',output]);
}
const esc=s=>s.replaceAll('&','&amp;').replaceAll('"','&quot;').replaceAll('<','&lt;');
let body='',animation='',voiceWindows=[];
scenes.forEach(([start,duration,kicker,title,accent,voice,rail,shots],i)=>{
 const id=`s${i+1}`,wide=i===1,focus=i>=2&&i<=4,dual=i===6,close=i===9;
 const heading=`<h1>${esc(title)}<span>${esc(accent)}</span></h1>`;
 body+=`<div id="${id}" class="scene ${wide?'wide':''} ${focus?'focus':''} ${dual?'dual':''} ${close?'close':''}">
 <div id="${id}-copy" class="copy clip" data-start="${start}" data-duration="${duration}" data-track-index="0"><div class="copy-inner"><p class="eyebrow">${esc(kicker)}</p>${heading}<p class="rail">${esc(rail)}</p>${close?'<p class="release">Get the Windows preview · Windows 11 · Unsigned</p>':''}</div></div>`;
 let cursor=start;
 shots.forEach(([name,length],j)=>{
 const at=dual?start:cursor;
 body+=`<div class="media-frame frame-${j}" data-layout-allow-overlap><div id="${id}-screen-${j}" class="screen"><video id="${id}-v${j}" class="clip" src="assets/clips/${name}-${length}s.mp4" muted playsinline preload="auto" data-start="${at}" data-duration="${length}" data-track-index="${dual?j+1:1}" aria-label="${esc(names[name])}"></video></div></div>`;
 cursor+=length;
 });
 if(!close)body+=`<div id="${id}-caption" class="caption clip" data-start="${start}" data-duration="${duration}" data-track-index="5"><p>${esc(voice)}</p></div>`;
 body+='</div>';
 animation+=`tl.fromTo('#${id} .copy-inner',{x:${wide?-80:-28},y:44,scale:.94,opacity:0},{x:0,y:0,scale:1,opacity:1,duration:.56,ease:'back.out(1.6)'},${start+.04});
 tl.fromTo('#${id} .media-frame',{x:${wide?0:88},scale:${wide?.9:.82},opacity:0},{x:0,scale:1,opacity:1,duration:.68,ease:'back.out(1.45)',stagger:.1},${start+.06});
 tl.fromTo('#${id} .caption p',{y:18,opacity:0},{y:0,opacity:1,duration:.3},${start+.2});
 `;
 if(wide){
  animation+=`tl.fromTo('#${id} .build-words b',{y:70,scale:.72,opacity:0,rotation:-5},{y:0,scale:1,opacity:1,rotation:0,duration:.42,ease:'back.out(1.8)',stagger:.22},${start+.18});
  tl.fromTo('#${id} .build-words span',{y:76,scale:.72,opacity:0,rotation:5},{y:0,scale:1,opacity:1,rotation:0,duration:.46,ease:'back.out(1.9)'},${start+1.05});
  tl.fromTo('#${id}-screen-0',{scale:1,x:22,y:8},{scale:1.045,x:-22,y:-8,duration:6.7,ease:'sine.inOut'},${start+.15});`;
 }else if(focus){
  shots.forEach(([name,length],j)=>{
   const at=shots.slice(0,j).reduce((sum,shot)=>sum+shot[1],start);
   const screen=`#${id}-screen-${j}`,endScale=1.035;
   const panX=name==='compressor'?42:name==='gate'?-22:name==='duck'?30:-18;
   const panY=name==='eq'?-18:name==='compressor'?8:-12;
   const first=Math.min(1.35,length*.42),second=Math.max(.3,length-first-.32);
   animation+=`tl.fromTo('${screen}',{scale:1.025,x:${-panX},y:${-panY}},{scale:${endScale},x:${panX},y:${panY},duration:${first},ease:'power2.inOut'},${at+.12});
   tl.to('${screen}',{scale:${endScale+.055},x:${-panX*.5},y:${-panY*.6},duration:${second},ease:'sine.inOut'},${at+.12+first});`;
  });
 }else{
  shots.forEach(([name,length],j)=>{
   const at=shots.slice(0,j).reduce((sum,shot)=>sum+shot[1],start);
   const screen=`#${id}-screen-${j}`,panning=name==='arrange';
   const first=Math.min(panning?3.3:1.65,length-.12),second=Math.max(.35,length-first-.25);
   animation+=`tl.fromTo('${screen}',{scale:${panning?1.12:1.025},x:${panning?230:24},y:${panning?0:12}},{scale:${panning?1.28:1.105},x:${panning?-230:-22},y:${panning?0:-10},duration:${first},ease:'${panning?'sine.inOut':'power1.inOut'}'},${at+.1});
   tl.to('${screen}',{scale:${panning?1.33:1.16},x:${panning?-270:18},y:${panning?0:4},duration:${second},ease:'sine.inOut'},${at+.1+first});`;
  });
 }
 const vf=`assets/audio/voice-${String(i+1).padStart(2,'0')}.wav`;
 const vd=Number(execFileSync('ffprobe',['-v','error','-show_entries','format=duration','-of','csv=p=0',vf],{encoding:'utf8'}).trim());
 const va=start+(close?.18:.4);
 body+=`<audio id="voice-${i+1}" src="${vf}" data-start="${va}" data-duration="${vd}" data-track-index="6" data-volume="0.85"></audio>`;
 voiceWindows.push([va,va+vd]);
});
const sampleDuration=Number(execFileSync('ffprobe',['-v','error','-show_entries','format=duration','-of','csv=p=0','assets/audio/voice-sample.wav'],{encoding:'utf8'}).trim());
body+=`<audio id="voice-sample" src="assets/audio/voice-sample.wav" data-start="7.45" data-duration="${sampleDuration}" data-track-index="6" data-volume="0.9"></audio>`;
voiceWindows.push([7.45,7.45+sampleDuration]);
const points=[{t:0,v:0}];
for(const [a,b] of voiceWindows){points.push({t:Math.max(.02,a-.15),v:.35},{t:a,v:.15},{t:b,v:.15},{t:Math.min(69.7,b+.18),v:.35});}
points.push({t:69.9,v:0});points.sort((a,b)=>a.t-b.t);
const automation=esc(JSON.stringify({version:1,lanes:[{target:'volume',points}]}));
body+=`<audio id="music" src="assets/audio/launch-original.wav" data-start="0" data-duration="70" data-track-index="7" data-volume="1" data-automation="${automation}"></audio>`;
const css=`*{box-sizing:border-box}html,body{margin:0;width:100%;height:100%;overflow:hidden;background:#10131a;color:#e9edf5;font-family:ui-sans-serif,system-ui,sans-serif}#root{position:relative;width:100%;height:100%;overflow:hidden}.scene{position:absolute;inset:0}.copy{position:absolute;left:64px;top:125px;width:700px;height:740px}.copy-inner{width:100%;height:100%}.eyebrow{font-size:24px;letter-spacing:.14em;font-weight:750;text-transform:uppercase;color:#9ba8bd;margin:0 0 44px}h1{font-size:100px;line-height:1.02;letter-spacing:-.055em;font-weight:900;margin:0;width:700px}h1 span{display:block;color:#25b8df;margin-top:12px}.rail{font-size:28px;line-height:1.45;font-weight:600;color:#9ba8bd;margin:48px 0 0;max-width:680px}.media-frame{position:absolute;left:810px;top:245px;width:1040px;height:585px;background:#141923;border:2px solid #25b8df;border-radius:20px;overflow:hidden;box-shadow:0 24px 50px #0008}.media-frame video{position:absolute;inset:0;width:100%;height:100%;object-fit:contain}.shot-label{position:absolute;left:0;right:0;bottom:0;padding:15px 20px;background:#10131af2;color:#e9edf5;font-size:24px;font-weight:750;height:62px}.caption{position:absolute;left:64px;right:64px;top:937px;height:95px;display:flex;align-items:center;justify-content:center;text-align:center}.caption p{font-size:29px;line-height:1.35;margin:0;max-width:1670px;color:#e9edf5;font-weight:500}.wide .copy{top:64px;height:225px;width:1792px}.wide .eyebrow{margin-bottom:18px}.wide h1{font-size:75px;width:1792px}.wide h1 span{display:inline;margin-left:24px}.wide .rail{display:none}.wide .media-frame{left:64px;top:325px;width:1792px;height:565px}.dual .copy{top:64px;width:1792px;height:220px}.dual .eyebrow{margin-bottom:18px}.dual h1{font-size:84px;width:1792px}.dual h1 span{display:inline;margin-left:24px}.dual .rail{margin-top:18px}.dual .media-frame{left:64px;top:365px;width:865px;height:500px}.dual .frame-1{left:991px}.close .copy{left:160px;top:205px;width:1600px;height:650px;text-align:center}.close h1{width:1600px;font-size:168px}.close h1 span{font-size:62px;line-height:1.2;max-width:1100px;margin:30px auto;color:#e9edf5;letter-spacing:-.035em}.close .rail{max-width:none;color:#25b8df;font-size:32px;margin-top:42px}.release{font-size:24px;color:#9ba8bd;margin-top:26px}#s5 h1 span,#s9 h1 span{color:#a37cff}#s9 .media-frame{height:610px;top:160px}#s9 .shot-label{height:62px}`;
const motionCss=`.screen{position:absolute;inset:0;transform-origin:center center}.screen video{position:absolute;inset:0;width:100%;height:100%}.wide .copy{top:30px;height:145px;width:1792px;text-align:center}.wide .eyebrow{margin-bottom:8px;font-size:19px}.wide h1{font-size:62px;width:1792px}.wide h1 b{display:inline-block;margin-right:18px}.wide h1 span{display:inline-block;margin-left:22px}.wide .media-frame{left:231px;top:175px;width:1458px;height:820px;border:0;border-radius:18px;box-shadow:none;background:transparent}.wide .media-frame video{border:0!important;border-radius:0!important;background:transparent!important}.wide .screen video{object-fit:contain}.wide .caption{top:995px;height:60px}.focus .copy{left:64px;top:240px;width:470px;height:570px}.focus .eyebrow{font-size:18px;line-height:1.35;letter-spacing:.11em;margin-bottom:26px}.focus h1{font-size:50px;line-height:1.04;letter-spacing:-.045em;width:470px}.focus h1 span{margin-top:10px}.focus .rail{font-size:18px;line-height:1.4;margin-top:30px;max-width:460px}.focus .media-frame{left:570px;top:190px;width:1280px;height:720px;border-radius:0}.focus .screen video{object-fit:contain}.focus .caption{left:64px;right:auto;top:790px;width:465px;height:220px;justify-content:flex-start;text-align:left}.focus .caption p{font-size:23px;line-height:1.3;max-width:460px}`;
// Frames are transparent; the framework hides each timed video and its label.
const visibilityFix='.media-frame{background:transparent!important;border:0;box-shadow:none!important}.media-frame video{background:transparent!important;border:0!important;border-radius:0!important}';
fs.writeFileSync('index.html',`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=1920,height=1080"><title>AudioRouter · See your sound</title><script src="assets/vendor/gsap.min.js"></script><style>${css}${motionCss}${visibilityFix}</style></head><body><div id="root" data-composition-id="main" data-width="1920" data-height="1080" data-duration="70">${body}</div><script>const tl=gsap.timeline({paused:true});${animation}window.__timelines['main']=tl;</script></body></html>`);
console.log('Authored 70-second composition with real AudioRouter footage, camera moves, local voice and music; no synthetic in-app graphics.');
