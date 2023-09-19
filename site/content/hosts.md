---
title: "hosts"
---
<div class="container mt-1 d-flex p-3 justify-content-between flex-wrap">
    <div style="display:flex;align-items: center;">
        <div class="form-check form-switch">
            <input class="form-check-input" type="checkbox" role="switch" id="staleHosts1" checked onClick="filterTypes('stale')" disabled>
            <label class="form-check-label" for="staleHosts1">Show Stale Hosts</label>
        </div>
        <div class="form-check form-switch ms-4">
            <input class="form-check-input" type="checkbox" role="switch" id="inactiveHosts1" checked onClick="filterTypes('inactive')" disabled>
            <label class="form-check-label" for="inactiveHosts1">Show Inactive Hosts</label>
        </div>
        <div class="form-check form-switch ms-4">
            <input class="form-check-input" type="checkbox" role="switch" id="inviteHosts1" checked onClick="filterTypes('invite')" disabled>
            <label class="form-check-label" for="inviteHosts1">Show Pending Invites</label>
        </div>
    </div>
    <div>
    <button type="button" class="btn btn-outline-primary" data-bs-toggle="modal" data-bs-target="#staticDownload"><i class="bi bi-download"></i> download Agent</button>
    <button type="button" class="btn btn-outline-primary" data-bs-toggle="modal" data-bs-target="#staticBackdrop" onClick="initAgent()"><i class="bi bi-plus-circle"></i> new Agent</button>
    </div>
</div>
<div class="container my-2" id="all"></div>
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
                    <a href="https://github.com/apimeister/unpatched-server/releases/latest/download/unpatched-server_x86_64-pc-windows-gnu.zip"><i class="bi bi-windows me-2"></i>unpatched-agent.exe</a>
                </div>
                <div class="mb-2">
                    <a href="https://github.com/apimeister/unpatched-server/releases/latest/download/unpatched-server_x86_64-apple-darwin.zip" ><i class="bi bi-apple me-2"></i>unpatched-agent</a>
                </div>
                <div class="mb-2">
                    <a href="https://github.com/apimeister/unpatched-server/releases/latest/download/unpatched-server_x86_64-unknown-linux-musl.tar.gz"><i class="bi bi-filetype-sh me-2"></i>unpatched-agent</a>
                </div>
            </div>
            <div class="modal-footer">
                <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Close</button>
            </div>
        </div>
    </div>
</div>
<div class="modal fade" id="staticExec" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticExecLabel" aria-hidden="true">
    <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
            <div class="modal-header">
                <h1 class="modal-title fs-5" id="staticExecLabel">Executions</h1>
                <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
            </div>
            <div class="modal-body" id="staticExecBody">
                Implement this
            </div>
            <div class="modal-footer">
                <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Close</button>
            </div>
        </div>
    </div>
</div>
<div class="modal fade" id="staticRun" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticRunLabel" aria-hidden="true">
    <div class="modal-dialog modal-dialog-centered modal-lg modal-dialog-scrollable">
        <div class="modal-content">
            <div class="modal-header">
                <h1 class="modal-title fs-5" id="staticRunLabel">Available Scripts</h1>
                <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
            </div>
            <div class="modal-body" id="staticRunBody">
                <ul class="list-group" id="staticRunUl"></ul>
            </div>
            <div class="modal-footer">
                <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Close</button>
            </div>
        </div>
    </div>
</div>
<div class="modal fade" id="staticRunNewScript" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticRunNewScriptLabel" aria-hidden="true">
    <div class="modal-dialog modal-dialog-centered modal-xl  modal-dialog-scrollable">
        <div class="modal-content">
            <div class="modal-header">
                <h1 class="modal-title fs-5" id="staticRunNewScriptLabel">Create a new Script and run it</h1>
                <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
            </div>
            <div class="modal-body" id="staticRunNewScriptBody">
                <div class="row">
                    <div class="col-md-6 col-12">
                        <form id="scriptFormModal" class="needs-validation" novalidate>
                            <div class="row mb-3">
                                <!-- info side left -->
                                <div class="col-md-6 col-12">
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptNameModal" class="form-label">Script Name</label>
                                            <input id="scriptNameModal" name="name" type="text" class="form-control" placeholder="My New Script" required>
                                            <div class="invalid-feedback">
                                                Please choose a name for your script
                                            </div>
                                        </div>
                                    </div>
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptVersionModal" class="form-label">Script Version</label>
                                            <input id="scriptVersionModal" name="version" type="text" class="form-control" placeholder="0.1.0">
                                        </div>
                                    </div>
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptLabelsModal" class="form-label">Labels (comma seperated)</label>
                                            <input id="scriptLabelsModal" name="labels" type="text" class="form-control" placeholder="linux,prod">
                                        </div>
                                    </div>
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptTimeoutModal" class="form-label">Timeout in seconds</label>
                                            <input id="scriptTimeoutModal" name="timeout" type="text" class="form-control" placeholder="5" oninput="timeModal(this.value)">
                                            <p id="scriptTimeoutHintModal">Info (readable): 0h:00m:05s</p>
                                        </div>
                                    </div>
                                    <div class="row mb-3">
                                        <div class="col">
                                        </div>
                                    </div>
                                </div>
                                <!-- script side right -->
                                <div class="col-md-6 col-12">
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptRegexModal" class="form-label">Output Regex</label>
                                            <textarea id="scriptRegexModal" name="output_regex" class="form-control" rows="3" placeholder=".*"></textarea>
                                        </div>
                                    </div>
                                        <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptContentModal" class="form-label">Script Content</label>
                                            <textarea id="scriptContentModal" name="script_content" class="form-control" rows="8" placeholder="uptime -p" required></textarea>
                                            <div class="invalid-feedback">
                                                Please enter something to execute
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </form>
                    </div>
                </div>
            </div>
            <div class="modal-footer">
                <button type="button" class="btn btn-primary" onClick="newScript(event)">Create and Run</button>
                <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Close</button>
            </div>
        </div>
    </div>
</div>
<script>
function filterTypes(type) {
    let d = document.getElementsByClassName(`hostCard ${type}`);
    for (e of d) { e.classList.toggle("d-none");}
}
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
function typing(agent){
    if (!agent.active) {return "inactive"};
    if (!agent.last_checkin) { return "invite"};
    let agent_time = online(agent.last_checkin);
    if (agent_time.parsed_time.hours > 1) {return "stale"};
    return "active"
}
async function init(){
    let agents = await fetch('/api/v1/hosts').then(r=>r.json());
    if (agents.error == "Invalid token") { window.location.href = "/login" }
    console.log(agents);
    let s = /*html*/`<div class="row row-cols-1 row-cols-sm-2 row-cols-md-3 g-4">`;
    for(agent of agents){
        const type = typing(agent);
        const time = online(agent.last_checkin);
        if (type == "stale") { document.getElementById("staleHosts1").removeAttribute("disabled"); }
        if (type == "inactive") { document.getElementById("inactiveHosts1").removeAttribute("disabled"); }
        if (type == "invite") { document.getElementById("inviteHosts1").removeAttribute("disabled"); }
        let atts="";
        for(attr of agent.attributes){
            atts+=/*html*/`<span class="badge rounded-pill text-bg-secondary me-1 ms-1">${attr}</span>`;
        }
        s += /*html*/`
        <div class="col row-flex hostCard ${type}" id="${agent.id}">
            <div class="card w-100">
                <div class="card-header" style="display: flex;justify-content: space-between;">
                    <div>${agent.alias || `Pending invite` }${type == "inactive" ? `<span class="fst-italic"> (deactivated)</span>`:``}</div>
                    <div>
                        <button class="btn btn-sm ${ type == "stale" ? `btn-warning`: type == "success" ? `btn-success`: `btn-secondary`} ${type == "invite" ? `opacity-0 pe-none`: ``}" onclick="${agent.active ? `deactivateHost(event)`:`activateHost(event)`}"><i class="bi bi-activity"></i></button>
                        <button class="btn btn-sm btn-outline-danger" onclick="deleteHost(event)"><i class="bi bi-trash"></i></button>
                    </div>
                </div>
                <div class="card-body">
                    <div class="card-text">Key: ${agent.id}</div>
                    <div class="card-text">Last check-in: ${ agent.last_checkin ? `<abbr title="${time.utcDBDate}">${time.parsed_time.readable_time}</abbr> ago` : `Never` }</div>
                    <div class="card-text">${atts || `No labels set`}</div>
                </div>
                <div class="card-body" style="display: flex;justify-content: space-around;">
                    <a class="icon-link icon-link-hover link-secondary ${type == "invite" ? `opacity-0 pe-none`:``}" href="#" onClick="runModal(event)" data-bs-toggle="modal" data-bs-target="#staticRun">Run Script <i class="bi bi-play-circle"></i></a>
                    <a class="icon-link icon-link-hover link-secondary ${type == "invite" ? `opacity-0 pe-none`:``}" href="#" onClick="execModal(event)" data-bs-toggle="modal" data-bs-target="#staticExec">Show Executions <i class="bi bi-search"></i></a>
                </div>
            </div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
async function runModal(evt){
    if(evt) evt.preventDefault();
    document.getElementById("staticRun").firstElementChild.id = evt.target.closest(".col").id;
    let scripts = await fetch('/api/v1/scripts').then(r=>r.json());
    console.log(scripts);
    let s = /*html*/`<li class="list-group-item d-flex justify-content-between"><span>Create a new Script and run it</span> <button type="button" class="btn btn-success px-3" data-bs-toggle="modal" data-bs-target="#staticRunNewScript"><i class="bi bi-plus-circle"></i></button></li>`;
    for (script of scripts) {
        s += /*html*/`
            <li class="list-group-item d-flex justify-content-between"><span>${script.name}</span> <span>v${script.version}</span> <button type="button" class="btn btn-primary" id="${script.id}" onClick="runNow(event)">Run</button></li>
        `
    }
    s += /*html*/``;
    document.getElementById("staticRunUl").innerHTML=s;
}
async function execModal(evt){
    if(evt) evt.preventDefault();
    let hostId = evt.target.closest(".col").id;
    let executions = await fetch(`/api/v1/hosts/${hostId}/executions`).then(r=>r.json());
    console.log(executions);
    let s = /*html*/`<ul>`;
    for (execution of executions) {
        s += /*html*/`
            <li class="list-group-item d-flex justify-content-between">${JSON.stringify(execution)}</li>
        `
    }
    s += /*html*/`</ul>`;
    document.getElementById("staticExecBody").innerHTML=s;
}
async function newScript(evt){
    let modalForm = document.getElementById("scriptFormModal");
    if (!modalForm.checkValidity()) {
        modalForm.classList.add('was-validated')
        return
      }
    let formData = new FormData(modalForm);
    let formDataObject = Object.fromEntries(formData.entries());
    formDataObject.labels = (formDataObject.labels || document.getElementById("scriptLabelsModal").placeholder).split(',');
    formDataObject.version = formDataObject.version || document.getElementById("scriptVersionModal").placeholder;
    formDataObject.timeout = { secs: parseInt(formDataObject.timeout || document.getElementById("scriptTimeoutModal").placeholder), nanos: 0 };
    formDataObject.output_regex = formDataObject.output_regex || document.getElementById("scriptRegexModal").placeholder;
    let formDataJsonString = JSON.stringify(formDataObject);
    let fetchOptions = {
        method: "POST",
        headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        },
        body: formDataJsonString,
    };
    let scriptId = await fetch('/api/v1/scripts', fetchOptions).then(r=>r.json());
    await createSchedule(scriptId);
    location.reload();
}
async function runNow(evt){
    await createSchedule(evt.target.id);
    location.reload();
}
async function createSchedule(scriptId){
    let hostId = document.getElementById("staticRun").firstElementChild.id;
    const schedule = {
        script_id: scriptId,
        target: {host_id: hostId},
        timer: {timestamp: new Date().toJSON()},
        active: true
    };
    let scheduleJsonString = JSON.stringify(schedule);
    let fetchOptions = {
        method: "POST",
        headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        },
        body: scheduleJsonString,
    };
    let res = await fetch('/api/v1/schedules', fetchOptions);
    if (!res.ok) {
        let error = await res.text();
        throw new Error(error);
    }
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
