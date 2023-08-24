---
title: "Executions"
---
<div class="container mt-4 mb-8" id="all"></div>
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
    let executions = await fetch('/api/v1/executions').then(r => r.json());
    console.log(executions);
    let s = /*html*/`<div class="row">
        <div class="header col" style="border-top-left-radius:1em;">id</div>
        <div class="header col">created</div>
        <div class="header col">host_id</div>
        <div class="header col">sched_id</div>
        <div class="header col">request</div>
        <div class="header col">response</div>
        <div class="header col" style="border-top-right-radius:1em;">output</div>
    </div>`;
    for(execution of executions){
        s += /*html*/`<div class="row">
            <div class="cell col">${execution.id}</div>
            <div class="cell col">${execution.created}</div>
            <div class="cell col">${execution.host_id}</div>
            <div class="cell col">${execution.sched_id}</div>
            <div class="cell col">${execution.request}</div>
            <div class="cell col">${execution.response}</div>
            <div class="cell col text-truncate">${execution.output}</div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>
