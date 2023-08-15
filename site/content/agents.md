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
    // Extract individual components from the timestamp
    const [datePart, timePart] = last_pong.split(' ');
    const [year, month, day] = datePart.split('-');
    const [hour, minute, second] = timePart.split(':');
    // Create a new UTC Date object using the extracted components
    const utcDBDate = new Date(Date.UTC(year, month - 1, day, hour, minute, second));
    const isoDBDate = utcDBDate.toISOString();
    const now = new Date(Date.now());
    const elapsed_int = now - utcDBDate;
    const elapsed = parse_time(elapsed_int);
    return { utcDBDate, isoDBDate, elapsed };
}
async function init(){
    let agents = await fetch('/api/v1/hosts').then(r=>r.json());
    console.log(agents);
    let s = "";
    for(agent of agents){
        const time = online(agent.last_pong);
        let atts="";
        for(attr of agent.attributes){
            atts+=`<span class="badge rounded-pill text-bg-secondary me-1 ms-1">${attr}</span>`;
        }
        s += `<div class="card ms-2 me-2" style="width:25em;">
        <div class="card-header">
            ${agent.alias}
        </div>
        <div class="card-body">
            <div class="card-text">${agent.id}</div>
            <div class="card-text">${atts}</div>
        </div>
        <div class="card-body" style="display: flex;justify-content: space-around;">
            <a href="#" class="card-link">Run Script</a>
            <a href="#" class="card-link">Show Executions</a>
        </div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>