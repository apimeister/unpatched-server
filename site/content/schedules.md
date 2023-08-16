---
title: "schedules"
---
<div class="container mt-4 mb-8" id="all" style="display:grid;grid-template-columns: 15em 1fr 1fr 15em 15em;"></div>
<style>
.header{
    font-weight: 600;
    background-color: var(--bs-secondary);
    text-align: center;
    padding-top: 0.5em;
    padding-bottom: 0.5em;
}
.cell{
    text-align:center;
    padding-top: 0.3em;
    padding-bottom: 0.3em;
    border-bottom: 1px solid var(--bs-secondary);
}
</style>
<script>
async function init(){
    let schedules = await fetch('/api/v1/schedules').then(r=>r.json());
    console.log(schedules);
    let s = `<div class="header" style="border-top-left-radius:1em;">cron</div>
            <div class="header">script</div>
            <div class="header">attributes</div>
            <div class="header">last activity</div>
            <div class="header" style="border-top-right-radius:1em;">next run</div>`;
    for(schedule of schedules){
        let scriptId = mangleId(schedule.script_id);
        s += `<div class="cell">${schedule.cron}</div>
        <div class="cell" id="${scriptId}">${schedule.script_id}</div>
        <div class="cell">${schedule.attributes}</div>
        <div class="cell">${schedule.last_pong}</div>
        <div class="cell"></div>`;
        fetchScript(schedule.script_id);
    }
    document.querySelector("#all").innerHTML=s;
}
function mangleId(id){
    id.replaceAll('-')
}
async function fetchScript(scriptId){
    let result = await fetch('/api/v1/scripts/'+scriptId).then(r=>r.json());
    // https://github.com/apimeister/unpatched-server/issues/23
    result = result[0];
    document.querySelector('#'+mangleId(scriptId)).innerHTML = `${result.name} <span class="badge rounded-pill text-bg-light">${result.version}</span>`;
}
init()
</script>