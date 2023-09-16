---
title: "login"
---
<div class="container min-vh-100">
  <div class="row align-items-center min-vh-100">
    <div class="col-md-6">
      <h1><i class="bi bi-bandaid"></i> Unpatched Server Login</h1>
      <form>
        <div class="form-outline mb-4">
          <input type="email" id="loginEmail1" class="form-control" name="client_id" required />
          <label class="form-label" for="loginEmail1">Email address</label>
        </div>
          <div class="form-outline mb-4">
          <input type="password" id="loginPw1" class="form-control" name="client_secret" required />
          <label class="form-label" for="loginPw1">Password</label>
        </div>
        <button type="button" class="btn btn-primary btn-block mb-4" onClick="login(this.form)">Sign in</button>
      </form>
    </div>
  </div>
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