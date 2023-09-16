---
title: "login"
---
<div class="col-12 col-md-6" style="align-self: center;margin-top: auto;margin-bottom: auto;border: 1px solid var(--bs-gray-200);border-radius:1em;padding:1em;">
  <h1 class="mb-4" style="text-align:center;font-weight: 600;background-color: var(--bs-gray-200);border-top-left-radius: 1rem;border-top-right-radius: 1rem;">Login</h1>
  <form>
    <div class="form-outline mb-4">
      <input type="email" id="loginEmail1" class="form-control" name="client_id" required />
      <label class="form-label" for="loginEmail1">Email address</label>
    </div>
      <div class="form-outline mb-4">
      <input type="password" id="loginPw1" class="form-control" name="client_secret" required />
      <label class="form-label" for="loginPw1">Password</label>
    </div>
    <div class="d-grid">
      <button type="button" class="btn btn-primary btn-block mb-4" onClick="login(this.form)">Sign in</button>
    </div>
  </form>
</div>
<script>
async function login(form){
    let formData = new FormData(form);
    let formDataObject = Object.fromEntries(formData.entries());
    let formDataJsonString = JSON.stringify(formDataObject);
    let fetchOptions = {
        method: "POST",
        headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        },
        body: formDataJsonString,
    };
    let res = await fetch('/api/v1/authorize', fetchOptions);
    if (!res.ok) {
        let error = await res.text();
        throw new Error(error);
    }
    window.location.href = "/agents";
}
</script>