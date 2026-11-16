// One-time migration: load the original JS site and dump every content/data global to content.json
const {chromium}=require('playwright');const fs=require('fs');
(async()=>{const b=await chromium.launch();const p=await b.newPage();
p.on('console',m=>{if(m.type()==='error')console.log('console:',m.text())});
await p.goto('file://'+process.argv[2]);await p.waitForTimeout(800);
const data=await p.evaluate(()=>{
 const rx=ZRX.map(([re,rep])=>[re.source,re.flags,typeof rep==='string'?rep:null]);
 const qref={};const keep=S.lang;for(const l of ['en','zh','both']){S.lang=l;qref[l]=QREF()}S.lang=keep;
 return {programs:PROGRAMS,mods:MODS,topics:TOPICS.map(t=>({id:t.id,mod:t.mod,wk:t.wk,title:t.title,zh:t.zh,sections:t.sections,terms:t.terms||null,worked:t.worked||null,recap:t.recap||null})),
  zt:ZT,cards:CARDLIST,quiz:QUIZ,zq:ZQ,zd:ZD,zrx:rx,fx:FX,zf:ZF,zg:ZG,anno:ANNO,presets:PRESETS,zp:ZP,
  chint:CHINT,ohint:OHINT,zchint:ZCHINT,zohint:ZOHINT,zkl:ZKL,zil:ZIL,libcat:LIBCAT,libg:LIBG,xsec:XSEC,xwork:XWORK,
  calcs:CALCS.map(c=>({id:c.id,title:c.title,topic:c.topic,seg:c.seg||null,fields:c.fields,text:c.text||null})),qref};
});
fs.writeFileSync(process.argv[3],JSON.stringify(data));console.log('topics',data.topics.length,'cards',data.cards.length,'quiz',data.quiz.length,'fx',data.fx.length,'calcs',data.calcs.length,'zd',Object.keys(data.zd).length,'zrx',data.zrx.length);
await b.close()})();
