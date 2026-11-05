// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

use crate::Animation;

/// Portable HTML uses a small JS text player because it has no WASM dependency.
pub fn html(project: &Animation) -> String {
    let data = project
        .to_json()
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e");
    let mut result = String::from(
        r#"<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>ApollinaireASCII text animation</title>
<style>
:root{color-scheme:dark}body{margin:24px;background:#000000;color:#FFFFF0;font:16px system-ui}
button{font:inherit;min-height:44px;padding:8px 16px;background:#66023C;color:#FFFFF0;border:1px solid #FFFFF0;cursor:pointer}
button:focus-visible,pre:focus-visible{outline:3px solid #FFD32A;outline-offset:4px}
pre{font:14px/1.2 ui-monospace,Consolas,monospace;white-space:pre;overflow:auto;max-width:100%;user-select:text}
</style>
<h1>ApollinaireASCII</h1><p>Selectable ASCII text. Copy it into any monospace editor.</p>
<button id="play" type="button">Play</button> <button id="step" type="button">Next frame</button>
<p id="count"></p><pre id="art" tabindex="0" aria-label="ASCII artwork"></pre>
<script type="application/json" id="project">"#,
    );
    result.push_str(&data);
    result.push_str(r#"</script><script>
'use strict';
const p=JSON.parse(document.getElementById('project').textContent);
const art=document.getElementById('art'),play=document.getElementById('play'),count=document.getElementById('count');
let index=0,playing=false,last=0,elapsed=0,completed=false;
function show(){art.textContent=p.frames[index].text;count.textContent='Frame '+(index+1)+' of '+p.frames.length;}
function pause(){playing=false;play.textContent='Play';}
play.onclick=()=>{if(playing){pause();return;}if(completed){index=0;elapsed=0;completed=false;show();}playing=true;play.textContent='Pause';last=0;};
document.getElementById('step').onclick=()=>{pause();index=(index+1)%p.frames.length;elapsed=0;completed=false;show();};
document.addEventListener('visibilitychange',()=>{if(document.hidden)pause();});
const motion=matchMedia('(prefers-reduced-motion: reduce)');
motion.addEventListener('change',()=>{if(motion.matches)pause();});
function tick(now){
 if(playing&&!document.hidden){
  if(last)elapsed+=now-last;
  while(playing&&elapsed>=p.frames[index].duration_ms){
   elapsed-=p.frames[index].duration_ms;
   if(index+1<p.frames.length)index++;else if(p.loop)index=0;else {pause();completed=true;}
   show();
  }
 }last=now;requestAnimationFrame(tick);
}
show();requestAnimationFrame(tick);
</script></html>"#);
    result
}
