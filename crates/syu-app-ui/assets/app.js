(()=>{
  const $=(s,r=document)=>r.querySelector(s), $$=(s,r=document)=>[...r.querySelectorAll(s)];
  const toast=(message)=>{const el=$('.toast');el.textContent=message;el.classList.add('show');setTimeout(()=>el.classList.remove('show'),2200)};
  function route(page,push=true){$$('[data-page]').forEach(el=>el.hidden=el.dataset.page!==page);$$('[data-route]').forEach(el=>el.classList.toggle('active',el.dataset.route===page));if(push){const u=new URL(location);u.searchParams.set('page',page);history.pushState({},'',u)}}
  function tab(group,name,push=true){$$(`[data-tab-group="${group}"]`).forEach(el=>el.classList.toggle('active',el.dataset.tab===name));$$(`[data-panel-group="${group}"]`).forEach(el=>el.hidden=el.dataset.panel!==name);if(push){const u=new URL(location);u.searchParams.set('tab',name);history.pushState({},'',u)}}
  $$('[data-route]').forEach(el=>el.addEventListener('click',()=>route(el.dataset.route)));
  $$('[data-tab-group]').forEach(el=>el.addEventListener('click',()=>tab(el.dataset.tabGroup,el.dataset.tab)));
  const overlay=$('.palette-overlay'), input=$('[data-palette-input]');
  $('[data-open-palette]').addEventListener('click',()=>{overlay.classList.add('open');input.focus()});
  overlay.addEventListener('click',e=>{if(e.target===overlay)overlay.classList.remove('open')});
  input.addEventListener('input',()=>{$$('.palette-result').forEach(el=>el.hidden=!el.textContent.toLowerCase().includes(input.value.toLowerCase()))});
  $$('[data-command-route]').forEach(el=>el.addEventListener('click',()=>{const page=el.dataset.commandRoute;route(page);if(el.dataset.commandTab)tab(page,el.dataset.commandTab);if(page==='settings'){window.SyuPreferences.settingsLayer('application');window.SyuPreferences.settingsPage('application','language')}overlay.classList.remove('open');const target=el.dataset.commandFocus&&$(`[data-focus-id="${el.dataset.commandFocus}"]`);target?.focus();target?.classList.add('focus-ring');setTimeout(()=>target?.classList.remove('focus-ring'),1800)}));
  addEventListener('keydown',e=>{if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==='k'){e.preventDefault();overlay.classList.toggle('open');if(overlay.classList.contains('open'))input.focus()}if(e.key==='Escape')overlay.classList.remove('open')});
  onpopstate=()=>location.reload();
  const p=new URL(location).searchParams;route(p.get('page')||'work',false);const page=p.get('page')||'work';if(p.get('tab'))tab(page,p.get('tab'),false);if(p.get('palette')==='1')overlay.classList.add('open');
})();
