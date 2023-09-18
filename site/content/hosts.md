---
title: "hosts"
---
<div class="container mt-1" style="padding-left:1em;padding-right:1em;padding-top:0.25em;padding-bottom:0.25em;display:flex;justify-content: space-between;">
    <div style="display:flex;align-items: center;">
        <div class="form-check form-switch">
            <input class="form-check-input" type="checkbox" role="switch" id="staleHosts1" checked>
            <label class="form-check-label" for="staleHosts1">Show Stale Hosts</label>
        </div>
        <div class="form-check form-switch ms-4">
            <input class="form-check-input" type="checkbox" role="switch" id="inactiveHosts1" checked>
            <label class="form-check-label" for="inactiveHosts1">Show Inactive Hosts</label>
        </div>
    </div>
    <div>
    <button type="button" class="btn btn-outline-primary" data-bs-toggle="modal" data-bs-target="#staticDownload"><i class="bi bi-download"></i> download Agent</button>
    <button type="button" class="btn btn-outline-primary" data-bs-toggle="modal" data-bs-target="#staticBackdrop" onClick="initAgent()"><i class="bi bi-plus-circle"></i> new Agent</button>
    </div>
</div>
<div class="container mt-4 mb-4" id="all"></div>
<div class="modal fade" id="staticBackdrop" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticBackdropLabel" aria-hidden="true">
    <div class="modal-dialog modal-dialog-centered modal-lg">
        <div class="modal-content">
        <div class="modal-header">
            <h1 class="modal-title fs-5" id="staticBackdropLabel">Add a new Agent</h1>
            <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" onClick="location.reload()"></button>
        </div>
        <div class="modal-body">
            <div class="form-outline mb-2">
                <input type="text" id="hostAddr1" class="form-control" name="hostAddr1" required/>
                <label class="form-label" for="hostAddr1">Host address 
                <a href="#" data-bs-toggle="tooltip" title="Can be URL like localhost:3000 or IP like 127.0.0.1:3000 or IPv6 like [::1]:3000">
                <i class="bi bi-info-circle"></i>
                </a>
            </label>
            </div>
            <div class="form-outline mb-2">
                <input type="text" id="hostAlias1" class="form-control" name="hostAlias1" required placeholder="linux,prod"/>
                <label class="form-label" for="hostAlias1">Host alias</label>
            </div>
            <div class="form-outline mb-4">
                <input type="text" id="hostAttr1" class="form-control" name="hostAttr1" required placeholder="new-agent-123"/>
                <label class="form-label" for="hostAttr1">Attributes (comma seperated)</label>
            </div>
            <div class="bg-secondary p-2" style="--bs-bg-opacity: .3;">
            <code id="newAgentScript1"></code>
            </div>
        </div>
        <div class="modal-footer">
            <button type="button" class="btn btn-secondary" data-bs-dismiss="modal" onClick="location.reload()">Close</button>
        </div>
        </div>
    </div>
</div>
<div class="modal fade" id="staticDownload" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticDownloadLabel" aria-hidden="true">
    <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
            <div class="modal-header">
                <h1 class="modal-title fs-5" id="staticDownloadLabel">Download Agent Software</h1>
                <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
            </div>
            <div class="modal-body">
                <div class="mb-2">
                    <a href="#"><i class="bi bi-windows me-2"></i>unpatched-agent.exe</a>
                </div>
                <div class="mb-2">
                    <a href="#"><i class="bi bi-apple me-2"></i>unpatched-agent</a>
                </div>
                <div class="mb-2">
                    <a href="#"><i class="bi bi-filetype-sh me-2"></i>unpatched-agent</a>
                </div>
            </div>
            <div class="modal-footer">
                <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Close</button>
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
    return { hours, minutes, seconds, readable_time };
}
function online(last_checkin){
    const utcDBDate = new Date(last_checkin);
    const now = new Date(Date.now());
    const elapsed_int = now - utcDBDate;
    const parsed_time = parse_time(elapsed_int);
    return { utcDBDate, parsed_time };
}
async function initAgent(){
    let res = await fetch(`/api/v1/hosts/new`, {method: "POST"});
    if (!res.ok) {
        let error = await res.text();
        throw new Error(error);
    }
    res = await res.json();
    console.log(res);
    let dat = document.getElementById("hostAttr1");
    let dad = document.getElementById("hostAddr1");
    let dal = document.getElementById("hostAlias1");
    let nas = document.getElementById("newAgentScript1");
    dad.placeholder = `${window.location.host}`;
    dad.addEventListener("keyup", () => {
        nas.innerText = `unpatched-agent --alias ${dal.value || dal.placeholder} --attributes ${dat.value || dat.placeholder} --id ${res.id} --server ${dad.value || dad.placeholder}`;
     });
    dat.addEventListener("keyup", () => {
        nas.innerText = `unpatched-agent --alias ${dal.value || dal.placeholder} --attributes ${dat.value || dat.placeholder} --id ${res.id} --server ${dad.value || dad.placeholder}`;
     });
     dal.addEventListener("keyup", () => {
        nas.innerText = `unpatched-agent --alias ${dal.value || dal.placeholder} --attributes ${dat.value || dat.placeholder} --id ${res.id} --server ${dad.value || dad.placeholder}`;
     });
    nas.innerText = `unpatched-agent --alias ${dal.placeholder} --attributes ${dat.placeholder} --id ${res.id} --server ${window.location.host}`;
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
        <div class="col row-flex" id="${agent.id}">
        <div class="card w-100">
        <div class="card-header" style="display: flex;justify-content: space-between;">
            <div>${agent.alias || `Pending invite` }${!agent.active ? `<span class="fst-italic"> (deactivated)</span>`:``}</div>
            <div>
                <button class="btn btn-sm ${agent.active && agent.last_checkin ? (time.parsed_time.hours > 1 ? `btn-warning`:`btn-success`):`btn-secondary`} ${!agent.last_checkin ? `opacity-0 pe-none`: ``}" onclick="${agent.active ? `deactivateHost(event)`:`activateHost(event)`}"><i class="bi bi-activity"></i></button>
                <button class="btn btn-sm btn-outline-danger" onclick="deleteHost(event)"><i class="bi bi-trash"></i></button>
            </div>
        </div>
        <div class="card-body">
            <div class="card-text">Key: ${agent.id}</div>
            <div class="card-text">Last check-in: ${ agent.last_checkin ? `<abbr title="${time.utcDBDate}">${time.parsed_time.readable_time}</abbr> ago` : `Never` }</div>
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
async function deleteHost(evt){
    if(evt) evt.preventDefault();
    let hostId = evt.target.closest(".col").id;
    await fetch(`/api/v1/hosts/${hostId}`, {method: "DELETE"});
    location.reload();
}
async function deactivateHost(evt){
    if(evt) evt.preventDefault();
    let hostId = evt.target.closest(".col").id;
    await fetch(`/api/v1/hosts/${hostId}/deactivate`, {method: "POST"});
    location.reload();
}
async function activateHost(evt){
    if(evt) evt.preventDefault();
    let hostId = evt.target.closest(".col").id;
    await fetch(`/api/v1/hosts/${hostId}/activate`, {method: "POST"});
    location.reload();
}
init()
</script>
<style>
.row-flex {
  display: flex;
  flex-wrap: wrap;
}
</style>
