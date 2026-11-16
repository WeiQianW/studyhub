const {chromium}=require('playwright');
(async()=>{for(const [n,u] of [['original','http://localhost:8766/index.html'],['rust','http://localhost:8765/web/index.html']]){
 const b=await chromium.launch();const ts=[];const heap=[];const sw=[];
 for(let i=0;i<5;i++){const p=await b.newPage();await p.route(/fonts\.g/,r=>r.abort());const t0=Date.now();await p.goto(u);await p.waitForSelector('#app .hero');ts.push(Date.now()-t0);
  const t1=Date.now();for(const t of ['learn','cards','quiz','calc','lab','formulas','extra','home'])await p.click(`#tabs [data-tab=${t}]`);sw.push(Date.now()-t1);
  heap.push(await p.evaluate(()=>performance.memory?Math.round(performance.memory.usedJSHeapSize/1e6):0));await p.close()}
 ts.sort((a,b)=>a-b);sw.sort((a,b)=>a-b);console.log(n,'load ms median',ts[2],'| 8 tab switches ms',sw[2],'| JS heap MB',heap[2]);await b.close()}})();
