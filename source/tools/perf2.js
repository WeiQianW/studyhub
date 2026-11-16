const {chromium}=require('playwright');
(async()=>{for(const [n,u] of [['original','http://localhost:8766/index.html'],['rust','http://localhost:8765/web/index.html']]){
 const b=await chromium.launch();const p=await b.newPage();await p.route(/fonts\.g/,r=>r.abort());await p.goto(u);await p.waitForSelector('#app .hero');
 const res={};for(let k=0;k<3;k++)for(const t of ['learn','cards','quiz','calc','lab','formulas','extra','home']){
  const ms=await p.evaluate(t=>{const t0=performance.now();document.querySelector(`#tabs [data-tab=${t}]`).click();return performance.now()-t0},t);(res[t]=res[t]||[]).push(Math.round(ms))}
 console.log(n,JSON.stringify(res));await b.close()}})();
