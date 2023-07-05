(async function getData() {
    const res = await fetch("api");

    try {
        const obj = await res.json();
        console.log(obj);
        document.getElementById("hostname").innerHTML = obj[0]['host_name'];
        document.getElementById("uptime").innerHTML = obj[0]['uptime'];
        document.getElementById("os-release").innerHTML = obj[0]['os_release'];
    } catch (error) {
        console.log(error);
    }
  })()