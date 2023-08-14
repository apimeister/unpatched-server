---
title: "agents"
---
# Agents

<div id="all"></div>
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
function online(last_pong){
    // Extract individual components from the timestamp
    const [datePart, timePart] = last_pong.split(' ');
    const [year, month, day] = datePart.split('-');
    const [hour, minute, second] = timePart.split(':');
    // Create a new UTC Date object using the extracted components
    const utcDBDate = new Date(Date.UTC(year, month - 1, day, hour, minute, second));
    const isoDBDate = utcDBDate.toISOString();
    const now = new Date(Date.now());
    const elapsed_int = now - utcDBDate;
    const elapsed = parse_time(elapsed_int);
    return { utcDBDate, isoDBDate, elapsed };
}
async function init(){
    let agents = await fetch('/api/v1/hosts').then(r=>r.json());
    console.log(agents);
    let s = "";
    for(agent of agents){
        const time = online(agent.last_pong);
        s += `
        <div>
            <div>Id: ${agent.id}</div>
            <div>Alias: ${agent.alias}</div>
            <div>Attributes: ${agent.attributes}</div>
            <div>Ip: ${agent.ip}</div>
            <div>
                <div>Local: ${time.utcDBDate}</div>
                <div>Utc: ${time.isoDBDate}</div>
                <div>Elapsed time: ${time.elapsed}</div>
            </div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>