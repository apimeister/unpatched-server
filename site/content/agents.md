---
title: "agents"
---
# Agents

<div id="all"></div>
<script>
async function init(){
    let agents = await fetch('/api/v1/hosts').then(r=>r.json());
    console.log(agents);
    let s = "";
    for(agent of agents){
        s += `<div>
        <div>Id: ${agent.id}</div>
        <div>Alias: ${agent.alias}</div>
        <div>Attributes: ${agent.attributes}</div>
        <div>Ip: ${agent.ip}</div>
        <div>Last heartbeat: ${agent.last_pong}</div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>