// Differential test: drive the original JS site and the Rust build the same way and compare #app HTML.
const {chromium}=require('playwright');const fs=require('fs');
const SEED=`(()=>{let s=12345;Math.random=()=>{s=(s*1103515245+12345)%2147483648;return s/2147483648}})();`;
const steps=JSON.parse(process.argv[2]||'[]');
async function run(url,lang){const b=await chromium.launch();const ctx=await b.newContext({viewport:{width:1280,height:900}});
 await ctx.addInitScript(SEED+`localStorage.setItem('dpsh_lang',JSON.stringify('${lang}'));`);
 const p=await ctx.newPage();const errs=[];p.on('pageerror',e=>errs.push(String(e)));p.on('console',m=>{if(m.type()==='error')errs.push(m.text())});
 await p.goto(url);await p.waitForSelector('#tabs button',{timeout:20000});await p.waitForTimeout(400);
 const out={};
 for(const st of steps){
  if(st.click){await p.click(st.click).catch(e=>errs.push('click '+st.click+' '+e.message.split('\n')[0]))}
  if(st.select){await p.selectOption(st.select[0],st.select[1]).catch(e=>errs.push('sel '+e.message.split('\n')[0]))}
  if(st.fill){await p.fill(st.fill[0],st.fill[1]).catch(e=>errs.push('fill '+e.message.split('\n')[0]))}
  if(st.js){await p.evaluate(st.js).catch(e=>errs.push('js '+e.message.split('\n')[0]))}
  if(st.key){await p.keyboard.press(st.key)}
  await p.waitForTimeout(st.wait||250);
  if(st.snap){out[st.snap]=await p.evaluate(()=>document.getElementById('app').innerHTML+'||NB||'+(document.querySelector('aside.nb')||{}).innerHTML+'||Y||'+Math.round(scrollY)+'||'+location.hash)}
 }
 await b.close();return {out,errs}}
(async()=>{const lang=process.env.LANG2||'both';
 const A=await run('http://localhost:8766/index.html',lang);const B=await run('http://localhost:8765/web/index.html',lang);
 console.log('orig errors',A.errs.slice(0,5));console.log('rust errors',B.errs.slice(0,5));
 const norm=s=>s.replace(/\s+/g,' ').replace(/> </g,'><').replace(/data-id="h[0-9a-z]+"/g,'').replace(/data-hid="h[0-9a-z]+"/g,'');
 for(const k of Object.keys(A.out)){const a=norm(A.out[k]),r=norm(B.out[k]||'');
  if(a===r){console.log('SAME',k,a.length);continue}
  let i=0;while(i<a.length&&a[i]===r[i])i++;
  console.log('DIFF',k,a.length,r.length,'at',i);console.log('  orig:',a.slice(Math.max(0,i-120),i+200));console.log('  rust:',r.slice(Math.max(0,i-120),i+200));
  fs.writeFileSync('/tmp/d_'+k+'_a.html',a);fs.writeFileSync('/tmp/d_'+k+'_r.html',r)}
})();
