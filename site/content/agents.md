---
title: "agents"
---
<div class="container mt-4 mb-4" style="display:flex;" id="all"></div>
<script>
function parse_time(inp) {
            const i = inp / 1000
            const hours = Math.floor(i / 3600);
            let minutes = Math.floor((i % 3600) / 60);
            minutes = minutes < 10 ? '0' + minutes : minutes;
            let seconds = Math.floor((i % 3600) % 60);
            seconds = seconds < 10 ? '0' + seconds : seconds;
            const readable_time = /*html*/`${hours}:${minutes}:${seconds}`;
            return readable_time;
        }
function online(last_pong){
    const utcDBDate = new Date(last_pong);
    const now = new Date(Date.now());
    const elapsed_int = now - utcDBDate;
    const elapsed = parse_time(elapsed_int);
    return { utcDBDate, elapsed };
}
const nodeplus = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-clipboard2-plus" viewBox="0 0 16 16">
  <path d="M9.5 0a.5.5 0 0 1 .5.5.5.5 0 0 0 .5.5.5.5 0 0 1 .5.5V2a.5.5 0 0 1-.5.5h-5A.5.5 0 0 1 5 2v-.5a.5.5 0 0 1 .5-.5.5.5 0 0 0 .5-.5.5.5 0 0 1 .5-.5h3Z"/>
  <path d="M3 2.5a.5.5 0 0 1 .5-.5H4a.5.5 0 0 0 0-1h-.5A1.5 1.5 0 0 0 2 2.5v12A1.5 1.5 0 0 0 3.5 16h9a1.5 1.5 0 0 0 1.5-1.5v-12A1.5 1.5 0 0 0 12.5 1H12a.5.5 0 0 0 0 1h.5a.5.5 0 0 1 .5.5v12a.5.5 0 0 1-.5.5h-9a.5.5 0 0 1-.5-.5v-12Z"/>
  <path d="M8.5 6.5a.5.5 0 0 0-1 0V8H6a.5.5 0 0 0 0 1h1.5v1.5a.5.5 0 0 0 1 0V9H10a.5.5 0 0 0 0-1H8.5V6.5Z"/>
</svg>`
const search = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-search" viewBox="0 0 16 16">
  <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1.007 1.007 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0z"/>
</svg>`
async function init(){
    let agents = await fetch('/api/v1/hosts').then(r=>r.json());
    console.log(agents);
    let s = "";
    for(agent of agents){
        const time = online(agent.last_pong);
        let atts="";
        for(attr of agent.attributes){
            atts+=/*html*/`<span class="badge rounded-pill text-bg-secondary me-1 ms-1">${attr}</span>`;
        }
        s += /*html*/`<div class="card ms-2 me-2" style="width:25em;">
        <div class="card-header">
            ${agent.alias}
        </div>
        <div class="card-body">
            <div class="card-text">${agent.id}</div>
            <div class="card-text">Last check-in: <abbr title="${time.utcDBDate}">${time.elapsed}</abbr> ago</div>
            <div class="card-text">${atts}</div>
        </div>
        <div class="card-body" style="display: flex;justify-content: space-around;">
            <a class="icon-link icon-link-hover link-secondary" href="#">Run Script ${nodeplus}</a>
            <a class="icon-link icon-link-hover link-secondary" href="#">Show Executions ${search}</a>
        </div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>
