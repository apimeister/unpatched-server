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
            l +=/*html*/`<span class="badge rounded-pill text-bg-info">${label}</span>`
        }
        s += /*html*/`
            <div class="accordion-item">
                <h2 class="accordion-header">
                    <button class="accordion-button" type="button" data-bs-toggle="collapse" data-bs-target="#x${script.id}Collapse" aria-expanded="true" aria-controls="x${script.id}Collapse">
                        ${script.name} <span class="badge rounded-pill text-bg-secondary">${script.version}</span> <span class="text-end">${l}</span>
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
                                            <label for="scriptTimeout" class="form-label"><abbr title="One of:[y,mon,w,d,h,m,s,ms], use + to combine">Timeout</abbr></label>
                                            <input id="scriptTimeout" name="timeout" type="text" class="form-control" value="${script.timeout}">
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
init()
// const sampleForm = document.getElementById("script-form");
// sampleForm.addEventListener("submit", async (e) => {
//   e.preventDefault();
//   let form = e.currentTarget;
//   let url = form.action;
//   try {
//     let responseData = await postFormFieldsAsJson({ url, formData });
//     let { serverDataResponse } = responseData;
//     console.log(serverDataResponse);
//   } catch (error) {
//     console.error(error);
//   }
// });
// async function postFormFieldsAsJson({ url, formData }) {
//   let formDataObject = Object.fromEntries(formData.entries());
//   let formDataJsonString = JSON.stringify(formDataObject);
//   console.log(formDataJsonString);
</script>
