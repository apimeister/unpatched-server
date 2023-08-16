---
title: "scripts"
---
<div class="container" id="all"></div>
<script>
async function init(){
    let scripts = await fetch('/api/v1/scripts').then(r=>r.json());
    console.log(scripts);
    let s = "";
    for(script of scripts){
        s += `<div class="container mt-2 mb-2 pt-3 pb-3" style="border: 1px solid var(--bs-secondary);border-radius:1em;">
        <div class="mb-3">
            <h5 style="background-color:#efefef;text-align:center;">${script.id}</h5>
            <h6>version: ${script.version}</h6>
        </div>
        <div class="mb-3">
            <label for="exampleFormControlInput1" class="form-label">Script Name</label>
            <input type="text" class="form-control" value="${script.name}">
        </div>
        <div class="mb-3">
            <label for="exampleFormControlTextarea1" class="form-label">Example textarea</label>
            <textarea class="form-control" id="exampleFormControlTextarea1" rows="3">${script.script_content}</textarea>
        </div>
        <div class="mb-3">
            <label for="exampleFormControlInput1" class="form-label">Output Regex</label>
            <input type="text" class="form-control" value="${script.output_regex}">
        </div>
        <div class="mb-3">
            <label for="exampleFormControlInput1" class="form-label">Labels</label>
            <input type="text" class="form-control" value="${script.labels}">
        </div>
        <div class="mb-3">
            <label for="exampleFormControlInput1" class="form-label">Timeout in Seconds</label>
            <input type="text" class="form-control" value="${script.timeout}">
        </div>
        <div class="me-4 ms-4" style="display:flex;justify-content: flex-end;">
            <div class="btn-group">
                <button class="btn btn-outline-primary" type="button">
                    save as patch version
                </button>
                <button type="button" class="btn btn-outline-primary dropdown-toggle dropdown-toggle-split" data-bs-toggle="dropdown" aria-expanded="false">
                    <span class="visually-hidden">Toggle Dropdown</span>
                </button>
                <ul class="dropdown-menu">
                    <li><a class="dropdown-item" href="#">as patch version</a></li>
                    <li><a class="dropdown-item" href="#">as minor version</a></li>
                    <li><a class="dropdown-item" href="#">as major version</a></li>
                </ul>
            </div>
        </div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>