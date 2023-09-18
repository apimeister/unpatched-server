---
title: "schedules"
---
<div class="container my-5" id="all"></div>
<style>
.header{
    font-weight: 600;
    background-color: var(--bs-secondary);
    text-align: center;
    padding-top: 0.5em;
    padding-bottom: 0.5em;
}
.cell{
    text-align:center;
    padding-top: 0.3em;
    padding-bottom: 0.3em;
    border-bottom: 1px solid var(--bs-secondary);
}
</style>
<script>
async function init(){
    let users = await fetch('/api/v1/users').then(r => r.json());
    console.log(users);
    if (users.error == "Invalid token") { window.location.href = "/login" }
    let s = /*html*/`<div class="row">
        <div class="header col" style="border-top-left-radius:1em;">id</div>
        <div class="header col">email</div>
        <div class="header col">roles</div>
        <div class="header col">active</div>
        <div class="header col" style="border-top-right-radius:1em;">created</div>
    </div>`;
    for(user of users){
        s += /*html*/`<div class="row">
            <div class="cell col">${user.id}</div>
            <div class="cell col">${user.email}</div>
            <div class="cell col">${user.roles}</div>
            <div class="cell col">${user.active}</div>
            <div class="cell col">${user.created}</div>
        </div>`;
    }
    document.querySelector("#all").innerHTML=s;
}
init()
</script>
