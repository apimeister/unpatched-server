---
title: "schedules"
---
# Schedules

<div id="all"></div>
<script>
async function init(){
    let schedules = await fetch('/api/v1/schedules').then(r=>r.json());
    console.log(schedules);
    let s = "";
    for(schedule of schedules){
        s += `<div>
        <div>id: ${schedule.id}</div>
        <div>script_id: ${schedule.script_id}</div>
        <div>attributes: ${schedule.attributes}</div>
        <div>cron: ${schedule.cron}</div>
        <div>active: ${schedule.last_pong}</div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>