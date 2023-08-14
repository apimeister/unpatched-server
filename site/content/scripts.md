---
title: "scripts"
---
# Scripts

<div id="all"></div>
<script>
async function init(){
    let scripts = await fetch('/api/v1/scripts').then(r=>r.json());
    console.log(scripts);
    let s = "";
    for(script of scripts){
        s += `<div>
        <div>id: ${script.id}</div>
        <div>name: ${script.name}</div>
        <div>version: ${script.version}</div>
        <div>output_regex: ${script.output_regex}</div>
        <div>labels: ${script.labels}</div>
        <div>timeout: ${script.timeout}</div>
        <div>script_content: ${script.script_content}</div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>