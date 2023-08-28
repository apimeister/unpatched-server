---
title: "scripts"
---
<div class="container" id="all"></div>
<script>
async function init(){
    let scripts = await fetch('/api/v1/scripts').then(r=>r.json());
    console.log(scripts);
    let s = `<div class="container"><div class="accordion" id="accordionScript">`;
    for(script of scripts){
        let l = "";
        for(label of script.labels) {
            l +=/*html*/`<span class="badge text-bg-success">${label}</span>&nbsp;`
        }
        s += /*html*/`
            <div class="accordion-item">
                <h2 class="accordion-header">
                    <button class="accordion-button" type="button" data-bs-toggle="collapse" data-bs-target="#x${script.id}Collapse" aria-expanded="true" aria-controls="x${script.id}Collapse">
                        ${script.name}&nbsp;<span class="badge text-bg-secondary">${script.version}</span>&nbsp;<span class="text-end">${l}</span>
                    </button>
                </h2>
                <div id="x${script.id}Collapse" class="accordion-collapse collapse" data-bs-parent="#accordionScript">
                    <div class="accordion-body">
                        <form id="scriptForm">
                            <div class="row mb-3">
                                <!-- info side left -->
                                <div class="col-md-6 col-12">
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptId" class="form-label">Script ID</label>
                                            <input id="scriptId" name="scriptId" type="text" readonly class="form-control-plaintext" value="${script.id}">
                                        </div>
                                    </div>
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptVersion" class="form-label">Script Version</label>
                                            <input id="scriptVersion" name="version" type="text" readonly class="form-control-plaintext" value="${script.version}">
                                        </div>
                                    </div>
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptName" class="form-label">Script Name</label>
                                            <input id="scriptName" name="name" type="text" class="form-control" value="${script.name}">
                                        </div>
                                    </div>
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptLabels" class="form-label">Labels</label>
                                            <input id="scriptLabels" name="labels" type="text" class="form-control" value="${script.labels}">
                                        </div>
                                    </div>
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptTimeout" class="form-label">Timeout in seconds</label>
                                            <input id="scriptTimeout" name="timeout" type="text" class="form-control" value="${script.timeout.secs}" oninput="time(this.value)">
                                            <p id="scriptTimeoutHint">Info (readable): ${parse_time(script.timeout.secs)}</p>
                                        </div>
                                    </div>
                                    <div class="row mb-3">
                                        <div class="col">
                                        </div>
                                    </div>
                                </div>
                                <!-- script side right -->
                                <div class="col-md-6 col-12">
                                    <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptRegex" class="form-label">Output Regex</label>
                                            <textarea id="scriptRegex" name="output_regex" class="form-control" rows="3">${script.output_regex}</textarea>
                                        </div>
                                    </div>
                                        <div class="row mb-3">
                                        <div class="col">
                                            <label for="scriptContent" class="form-label">Script Content</label>
                                            <textarea id="scriptContent" name="script_content" class="form-control" rows="8">${script.script_content}</textarea>
                                        </div>
                                    </div>
                                </div>
                            </div>
                            <div class="row mb-3">
                                <div class="col text-end">
                                    <div class="btn-group">
                                        <button class="btn btn-outline-primary" type="button" onClick="sendScript(this.form, 'patch')">
                                            save as patch version
                                        </button>
                                        <button type="button" class="btn btn-outline-primary dropdown-toggle dropdown-toggle-split" data-bs-toggle="dropdown" aria-expanded="false">
                                            <span class="visually-hidden">Toggle Dropdown</span>
                                        </button>
                                        <ul class="dropdown-menu">
                                            <li><button class="dropdown-item" type="button" onClick="sendScript(this.form, 'patch')">as patch version</button></li>
                                            <li><button class="dropdown-item" type="button" onClick="sendScript(this.form, 'minor')">as minor version</button></li>
                                            <li><button class="dropdown-item" type="button" onClick="sendScript(this.form, 'major')">as major version</button></li>
                                        </ul>
                                    </div>
                                </div>
                            </div>
                        </form>
                    </div>
                </div>
            </div>
        `;
    }
    s +=`</div></div>`
    document.querySelector("#all").innerHTML=s;
}
async function sendScript(form, semver){
    let formData = new FormData(form);
    let formDataObject = Object.fromEntries(formData.entries());
    delete formDataObject.scriptId;
    formDataObject.labels = formDataObject.labels.split(',');
    let version_arr = formDataObject.version.split('.');
    switch (semver) {
        case 'patch':
            version_arr[2] = parseInt(version_arr[2]) + 1;
            break;
        case 'minor':
            version_arr[1] = parseInt(version_arr[1]) + 1;
            break;
        case 'major':
            version_arr[0] = parseInt(version_arr[0]) + 1;
            break;
        }
    formDataObject.timeout = { secs: parseInt(formDataObject.timeout), nanos: 0 };
    formDataObject.version = version_arr.join('.');
    let formDataJsonString = JSON.stringify(formDataObject);
    console.log(formDataJsonString);
    let fetchOptions = {
        method: "POST",
        headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        },
        body: formDataJsonString,
    };
    let res = await fetch('/api/v1/scripts', fetchOptions);
    if (!res.ok) {
        let error = await res.text();
        throw new Error(error);
    }
    return res.json();
}
function parse_time(inp) {
            const hours = Math.floor(inp / 3600);
            let minutes = Math.floor((inp % 3600) / 60);
            minutes = minutes < 10 ? '0' + minutes : minutes;
            let seconds = Math.floor((inp % 3600) % 60);
            seconds = seconds < 10 ? '0' + seconds : seconds;
            const readable_time = /*html*/`${hours}h:${minutes}m:${seconds}s`;
            return readable_time;
        }
function time(seconds) {
    const hint = document.getElementById("scriptTimeoutHint");
    const pretty_time = parse_time(seconds);
    hint.innerHTML = `Info (readable): ${pretty_time}`;
}
init();
</script>
