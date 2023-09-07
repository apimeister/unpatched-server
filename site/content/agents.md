---
title: "agents"
---
<link rel="stylesheet" href="/bootstrap-icons/1.10/bootstrap-icons.css">
<div class="container mt-1" style="background-color: var(--bs-gray-100);;border-radius:0.5em;padding-left:1em;padding-right:1em;padding-top:0.25em;padding-bottom:0.25em;display:flex;justify-content: space-between;">
    <div style="display:flex;align-items: center;">
        <div class="form-check form-switch">
            <input class="form-check-input" type="checkbox" role="switch" id="flexSwitchCheckChecked" checked>
            <label class="form-check-label" for="flexSwitchCheckChecked">Show Stale Agents</label>
        </div>
        <div class="form-check form-switch ms-4">
            <input class="form-check-input" type="checkbox" role="switch" id="flexSwitchCheckChecked" checked>
            <label class="form-check-label" for="flexSwitchCheckChecked">Show Inactive Agents</label>
        </div>
    </div>
    <button type="button" class="btn btn-outline-primary" data-bs-toggle="modal" data-bs-target="#staticBackdrop" onClick="initAgent()"><i class="bi bi-plus-circle"></i> new Agent</button>
</div>
<div class="container mt-4 mb-4" id="all"></div>
<div class="modal fade" id="staticBackdrop" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticBackdropLabel" aria-hidden="true">
    <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
        <div class="modal-header">
            <h1 class="modal-title fs-5" id="staticBackdropLabel">Add a new Agent</h1>
            <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" onClick="location.reload()"></button>
        </div>
        <div class="modal-body"><code id="newAgentScript1"></code>
        </div>
        <div class="modal-footer">
            <button type="button" class="btn btn-secondary" data-bs-dismiss="modal" onClick="location.reload()">Close</button>
        </div>
        </div>
    </div>
</div>
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
function online(last_checkin){
    const utcDBDate = new Date(last_checkin);
    const now = new Date(Date.now());
    const elapsed_int = now - utcDBDate;
    const elapsed = parse_time(elapsed_int);
    return { utcDBDate, elapsed };
}
async function initAgent(){
    let res = await fetch(`/api/v1/hosts/new`, {method: "POST"});
    if (!res.ok) {
        let error = await res.text();
        throw new Error(error);
    }
    res = await res.json();
    console.log(res);
    document.getElementById("newAgentScript1").innerText = `SSL_CERT_FILE=rootCA.crt unpatched-agent --alias new-agent-123 --attributes linux,prod --id ${res.id} --server ${window.location.host}`;
}
async function init(){
    let agents = await fetch('/api/v1/hosts').then(r=>r.json());
    if (agents.error == "Invalid token") { window.location.href = "/login" }
    console.log(agents);
    let s = /*html*/`<div class="row row-cols-1 row-cols-sm-2 row-cols-md-3 g-4">`;
    for(agent of agents){
        const time = online(agent.last_checkin);
        let atts="";
        for(attr of agent.attributes){
            atts+=/*html*/`<span class="badge rounded-pill text-bg-secondary me-1 ms-1">${attr}</span>`;
        }
        s += /*html*/`
        <div class="col row-flex">
        <div class="card w-100">
        <div class="card-header">
            ${agent.alias || `Pending invite` }
        </div>
        <div class="card-body">
            <div class="card-text">Key: ${agent.id}</div>
            <div class="card-text">Last check-in: ${ agent.last_checkin ? `<abbr title="${time.utcDBDate}">${time.elapsed}</abbr> ago` : `Never` }</div>
            <div class="card-text">${atts || `No labels set`}</div>
        </div>
        <div class="card-body" style="display: flex;justify-content: space-around;">
            <a class="icon-link icon-link-hover link-secondary ${agent.last_checkin ? ``:`opacity-0 pe-none`}" href="#">Run Script <i class="bi bi-clipboard2-plus"></i></a>
            <a class="icon-link icon-link-hover link-secondary ${agent.last_checkin ? ``:`opacity-0 pe-none`}" href="#" data-bs-toggle="modal" data-bs-target="#staticBackdrop2">Show Executions <i class="bi bi-search"></i></a>
        </div>
        </div>
        <div class="modal fade" id="staticBackdrop2" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticBackdropLabel2" aria-hidden="true">
        <div class="modal-dialog modal-dialog-centered">
            <div class="modal-content">
            <div class="modal-header">
                <h1 class="modal-title fs-5" id="staticBackdropLabel2">Executions for Agent ${agent.alias}</h1>
                <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
            </div>
            <div class="modal-body">
                Implement this
            </div>
            <div class="modal-footer">
                <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Close</button>
                <button type="button" class="btn btn-primary">Understood</button>
            </div>
            </div>
        </div>
        </div></div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>
<style>
.row-flex {
  display: flex;
  flex-wrap: wrap;
}
</style>
