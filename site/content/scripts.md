---
title: "scripts"
---
<div class="container mt-1 d-flex p-3 justify-content-end flex-wrap">
    <div><!--left side buttons here--></div>
    <div>
        <button type="button" class="btn btn-outline-primary" data-bs-toggle="modal" data-bs-target="#staticBackdrop"><i class="bi bi-plus-circle"></i> new Script</button>
        <a class="btn btn-outline-primary" href="/api/v1/scripts" download="scripts.json"><i class="bi bi-download"></i> Export Scripts</a>
    </div>
</div>
<div class="container my-2" id="all"></div>
<div class="modal fade" id="staticBackdrop" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticBackdropLabel" aria-hidden="true">
    <div class="modal-dialog modal-dialog-centered modal-lg">
        <div class="modal-content">
        <div class="modal-header">
            <h1 class="modal-title fs-5" id="staticBackdropLabel">Add a new Script</h1>
            <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
        </div>
        <div class="modal-body">
            <form id="scriptFormModal" class="needs-validation" novalidate>
                <div class="row mb-3">
                    <!-- info side left -->
                    <div class="col-md-6 col-12">
                        <div class="row mb-3">
                            <div class="col">
                                <label for="scriptNameModal" class="form-label">Script Name</label>
                                <input id="scriptNameModal" name="name" type="text" class="form-control" placeholder="My New Script" required>
                                <div class="invalid-feedback">
                                    Please choose a name for your script
                                </div>
                            </div>
                        </div>
                        <div class="row mb-3">
                            <div class="col">
                                <label for="scriptVersionModal" class="form-label">Script Version</label>
                                <input id="scriptVersionModal" name="version" type="text" class="form-control" placeholder="0.1.0">
                            </div>
                        </div>
                        <div class="row mb-3">
                            <div class="col">
                                <label for="scriptLabelsModal" class="form-label">Labels (comma seperated)</label>
                                <input id="scriptLabelsModal" name="labels" type="text" class="form-control" placeholder="linux,prod">
                            </div>
                        </div>
                        <div class="row mb-3">
                            <div class="col">
                                <label for="scriptTimeoutModal" class="form-label">Timeout in seconds</label>
                                <input id="scriptTimeoutModal" name="timeout" type="text" class="form-control" placeholder="5" oninput="timeModal(this.value)">
                                <p id="scriptTimeoutHintModal">Info (readable): 0h:00m:05s</p>
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
                                <label for="scriptRegexModal" class="form-label">Output Regex</label>
                                <textarea id="scriptRegexModal" name="output_regex" class="form-control" rows="3" placeholder=".*"></textarea>
                            </div>
                        </div>
                            <div class="row mb-3">
                            <div class="col">
                                <label for="scriptContentModal" class="form-label">Script Content</label>
                                <textarea id="scriptContentModal" name="script_content" class="form-control" rows="8" placeholder="uptime -p" required></textarea>
                                <div class="invalid-feedback">
                                    Please enter something to execute
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </form>
        </div>
        <div class="modal-footer">
            <button type="button" class="btn btn-primary" onClick="newScript()">Save and Close</button>
            <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Close</button>
        </div>
        </div>
    </div>
</div>
<script>
async function init(){
    let scripts = await fetch('/api/v1/scripts').then(r=>r.json());
    if (scripts.error == "Invalid token") { window.location.href = "/login" }
    console.log(scripts);
    let s = /*html*/`<div class="container"><div class="accordion" id="accordionScript">`;
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
                                        <button class="btn btn-outline-primary" type="button" onClick="updateScript(this.form, 'patch')">
                                            save as patch version
                                        </button>
                                        <button type="button" class="btn btn-outline-primary dropdown-toggle dropdown-toggle-split" data-bs-toggle="dropdown" aria-expanded="false">
                                            <span class="visually-hidden">Toggle Dropdown</span>
                                        </button>
                                        <ul class="dropdown-menu">
                                            <li><button class="dropdown-item" type="button" onClick="updateScript(this.form, 'patch')">as patch version</button></li>
                                            <li><button class="dropdown-item" type="button" onClick="updateScript(this.form, 'minor')">as minor version</button></li>
                                            <li><button class="dropdown-item" type="button" onClick="updateScript(this.form, 'major')">as major version</button></li>
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
async function updateScript(form, semver){
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
    location.reload();
}
async function newScript(){
    let modalForm = document.getElementById("scriptFormModal");
    if (!modalForm.checkValidity()) {
        modalForm.classList.add('was-validated')
        return
      }
    let formData = new FormData(modalForm);
    let formDataObject = Object.fromEntries(formData.entries());
    formDataObject.labels = (formDataObject.labels || document.getElementById("scriptLabelsModal").placeholder).split(',');
    formDataObject.version = formDataObject.version || document.getElementById("scriptVersionModal").placeholder;
    formDataObject.timeout = { secs: parseInt(formDataObject.timeout || document.getElementById("scriptTimeoutModal").placeholder), nanos: 0 };
    formDataObject.output_regex = formDataObject.output_regex || document.getElementById("scriptRegexModal").placeholder;
    let formDataJsonString = JSON.stringify(formDataObject);
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
    location.reload();
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
function timeModal(seconds) {
    const hint = document.getElementById("scriptTimeoutHintModal");
    const pretty_time = parse_time(seconds);
    hint.innerHTML = `Info (readable): ${pretty_time}`;
}
init();
</script>
