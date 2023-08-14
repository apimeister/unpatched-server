---
title: "executions"
---
# Scripts

<div id="all"></div>
<script>
async function init(){
    let executions = await fetch('/api/v1/executions').then(r=>r.json());
    console.log(executions);
    let s = "";
    for(execution of executions){
        s += `<div>
        <div>id: ${execution.id}</div>
        <div>request: ${execution.request}</div>
        <div>response: ${execution.response}</div>
        <div>host_id: ${execution.host_id}</div>
        <div>script_id: ${execution.script_id}</div>
        <div>sched_id: ${execution.sched_id}</div>
        <div>created: ${execution.created}</div>
        <div>output: ${execution.output}</div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>