import path from 'node:path';
import { readFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';

export async function savedToken() {
  if(process.platform!=='win32'||!process.env.LOCALAPPDATA)return null;
  const location=path.join(process.env.LOCALAPPDATA,'AudioRouter','api-token.dpapi');
  try {await readFile(location);}catch(e){if(e.code==='ENOENT')return null;throw new Error('Cannot read saved API token.');}
  // No token or caller-controlled path in process arguments. DPAPI decrypts
  // only for the logged-in user. Stdout stays private to this parent process.
  const command="$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Security; $p=Join-Path $env:LOCALAPPDATA 'AudioRouter/api-token.dpapi'; $b=[Security.Cryptography.ProtectedData]::Unprotect([IO.File]::ReadAllBytes($p),$null,[Security.Cryptography.DataProtectionScope]::CurrentUser); [Console]::Write([Text.Encoding]::UTF8.GetString($b))";
  const child=spawn('powershell.exe',['-NoProfile','-NonInteractive','-Command',command],{windowsHide:true,stdio:['ignore','pipe','ignore']});
  return new Promise((resolve,reject)=>{
    let value='';const timer=setTimeout(()=>{child.kill();reject(new Error('Saved API token could not be decrypted.'));},5000);
    child.stdout.on('data',b=>{value+=b;if(value.length>128){child.kill();}});
    child.once('error',()=>{clearTimeout(timer);reject(new Error('Cannot decrypt saved API token.'));});
    child.once('exit',code=>{clearTimeout(timer);if(code===0&&/^[a-fA-F0-9]{64}$/.test(value))resolve(value);else reject(new Error('Cannot decrypt saved API token. Reveal or regenerate it in AudioRouter.'));});
  });
}
