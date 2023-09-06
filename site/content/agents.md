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
const nodeplus = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-clipboard2-plus" viewBox="0 0 16 16">
  <path d="M9.5 0a.5.5 0 0 1 .5.5.5.5 0 0 0 .5.5.5.5 0 0 1 .5.5V2a.5.5 0 0 1-.5.5h-5A.5.5 0 0 1 5 2v-.5a.5.5 0 0 1 .5-.5.5.5 0 0 0 .5-.5.5.5 0 0 1 .5-.5h3Z"/>
  <path d="M3 2.5a.5.5 0 0 1 .5-.5H4a.5.5 0 0 0 0-1h-.5A1.5 1.5 0 0 0 2 2.5v12A1.5 1.5 0 0 0 3.5 16h9a1.5 1.5 0 0 0 1.5-1.5v-12A1.5 1.5 0 0 0 12.5 1H12a.5.5 0 0 0 0 1h.5a.5.5 0 0 1 .5.5v12a.5.5 0 0 1-.5.5h-9a.5.5 0 0 1-.5-.5v-12Z"/>
  <path d="M8.5 6.5a.5.5 0 0 0-1 0V8H6a.5.5 0 0 0 0 1h1.5v1.5a.5.5 0 0 0 1 0V9H10a.5.5 0 0 0 0-1H8.5V6.5Z"/>
</svg>`
const search = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-search" viewBox="0 0 16 16">
  <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1.007 1.007 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0z"/>
</svg>`
const plus = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-plus-circle" viewBox="0 0 16 16">
  <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14zm0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16z"/>
  <path d="M8 4a.5.5 0 0 1 .5.5v3h3a.5.5 0 0 1 0 1h-3v3a.5.5 0 0 1-1 0v-3h-3a.5.5 0 0 1 0-1h3v-3A.5.5 0 0 1 8 4z"/>
</svg>`
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
        <div class="col">
        <div class="card">
        <div class="card-header">
            ${agent.alias || `<span class="opacity-0">placeholder<span>` }
        </div>
        <div class="card-body">
            <div class="card-text">${agent.id || `<span class="opacity-0">placeholder<span>`}</div>
            <div class="card-text">Last check-in: ${ agent.last_checkin ? `<abbr title="${time.utcDBDate}">${time.elapsed}</abbr> ago` : `Never` }</div>
            <div class="card-text">${atts || `<span class="opacity-0">placeholder<span>`}</div>
        </div>
        <div class="card-body" style="display: flex;justify-content: space-around;">
            <a class="icon-link icon-link-hover link-secondary" href="#">Run Script ${nodeplus}</a>
            <a class="icon-link icon-link-hover link-secondary" href="#" data-bs-toggle="modal" data-bs-target="#staticBackdrop2">Show Executions ${search}</a>
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
    s += /*html*/`<div class="w-100"></div><div class="col"><div class="card">
        <div class="card-body bg-secondary" style="--bs-bg-opacity: .3;">
            <br><br><br><div class="card-text"><button type="button" class="btn btn-success position-absolute top-50 start-50 translate-middle" data-bs-toggle="modal" data-bs-target="#staticBackdrop" onClick="initAgent()">${plus} new Agent</button><br><br></div>
        </div>
        </div></div></div>
        `
    s += /*html*/`<div class="modal fade" id="staticBackdrop" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticBackdropLabel" aria-hidden="true">
        <div class="modal-dialog modal-dialog-centered">
            <div class="modal-content">
            <div class="modal-header">
                <h1 class="modal-title fs-5" id="staticBackdropLabel">Add a new Agent</h1>
                <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
            </div>
            <div class="modal-body"><code id="newAgentScript1"></code>
            </div>
            <div class="modal-footer">
                <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Close</button>
                <button type="button" class="btn btn-primary">Understood</button>
            </div>
            </div>
        </div>
        </div>
    `
    document.querySelector("#all").innerHTML=s;
}
init()
</script>
