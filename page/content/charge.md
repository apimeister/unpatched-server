---
title: "Billing - DownToZero"
---
<div id="content" style="display: flex;font-family: monospace;flex-flow: wrap;justify-content: center;flex-grow: 1;align-content: center;">
generating url...
</div>
<script>
function init(){
    let baseUrl = `https://buy.stripe.com/test_cN2dRPdw7fxg5lCfYY?client_reference_id=${user.sub}&prefilled_email=jens@apimeister.com`;
    let a = document.createElement("a");
    a.href = baseUrl;
    a.innerText="charge with stripe";
    document.querySelector('#content').innerHTML='';
    document.querySelector('#content').append(a);
}
init();
</script>