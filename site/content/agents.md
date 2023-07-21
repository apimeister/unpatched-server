---
title: "agents"
---
# Agents
<div id="all"></div>
<script>
async function init(){
    let agents = await fetch('/api/v1/agents').then(r=>r.json());
    console.log(agents);
    /*AgentData {
        id: d.get::<String, _>("id"),
        alias: d.get::<String, _>("name"),
        uptime: d.get::<i64, _>("uptime"),
        os_release: d.get::<String, _>("os_release"),
    };*/
    let s = "";
    for(agent of agents){
        s += `<div>
        <div>${agent.id}</div>
        <div>${agent.alias}</div>
        <div>${agent.uptime}</div>
        <div>${agent.os_release}</div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>