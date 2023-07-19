---
title: "Billing - DownToZero"
---
<div id="content" style="display: flex;font-family: monospace;flex-flow: wrap;justify-content: center;">
loading statistics...
</div>
<script>
let locale = Intl.NumberFormat().resolvedOptions().locale;
let MONEYFORMAT = new Intl.NumberFormat(locale, { style: 'currency', currency: 'EUR' });
function load(){
    fetch("/api/2022-12-28/stats").then(r=>r.json()).then(data=>{
        let card = `<div style="margin:1em;width:27em;border-radius: 2em;padding: 1em;border: 1px solid grey;">
            <h5 style="background-color:#efefef;border-radius: 1em;text-align:center;">Billing Statistics</h5>
            <div class="mt-2" style="display:flex;justify-content: space-between;">
            <h5 class="card-title">last update</h5>
            <div>
              <p class="card-text mb-0" style="text-align:right;">${data.ts.substring(0,10)}</p>
              <p class="card-text" style="text-align:right;">${data.ts.substring(11,19)}</p>
            </div>
            </div>
            <div class="mt-2" style="display:flex;justify-content: space-between;">
            <h5 class="card-title">balance</h5>
            <p class="card-text" style="text-align:right;">${MONEYFORMAT.format(data.value)}</p>
            </div>
        </div>`;
        document.getElementById("content").innerHTML = card;
    }).catch(err=> {
      if(err.message == "unauthorized"){
        let card = `<div style="color: var(--bs-danger);">unauthorized</div>`;
        document.getElementById("content").innerHTML = card;
      }else{
        console.log("something went wrong");
        console.log(err);
      }
    });
}
load();
</script>