---
title: "schedules"
---
<div class="container my-5" id="all"></div>
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
async function fetchScript(scriptId){
    let script = await fetch('/api/v1/scripts/' + scriptId).then(r => r.json());
    console.log(script);
    return /*html*/`${script.name} <span class="badge text-bg-secondary">${script.version}</span>`;
}
async function init(){
    let schedules = await fetch('/api/v1/schedules').then(r => r.json());
    if (schedules.error == "Invalid token") { window.location.href = "/login" }
    console.log(schedules);
    let s = /*html*/`<div class="row">
        <div class="header col" style="border-top-left-radius:1em;">id</div>
        <div class="header col">script</div>
        <div class="header col">timer</div>
        <div class="header col">target</div>
        <div class="header col">last activity</div>
        <div class="header col" style="border-top-right-radius:1em;">next run</div>
    </div>`;
    for(schedule of schedules){
        s += /*html*/`<div class="row">
            <div class="cell col">${schedule.id}</div>
            <div class="cell col" id="x${schedule.script_id}">${await fetchScript(schedule.script_id)}</div>
            <div class="cell col">${schedule.timer.cron || schedule.timer.timestamp}</div>
            <div class="cell col">${schedule.target.host_id || schedule.target.attributes}</div>
            <div class="cell col">${schedule.last_execution}</div>
            <div class="cell col"></div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>
