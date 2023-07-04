var loc = window.location, new_uri;
if (loc.protocol === "https:") {
    new_uri = "wss:";
} else {
    new_uri = "ws:";
}
new_uri += "//" + loc.host;
new_uri += loc.pathname + "ws";

const socket = new WebSocket(new_uri);

socket.addEventListener('open', function (event) {
    socket.send('Hello Server!');
});

socket.addEventListener('message', function (event) {
    
    try {
        const obj = JSON.parse(event.data);
        console.log(obj);
        document.getElementById("hostname").innerHTML = obj['hostname'];
        document.getElementById("uptime").innerHTML = obj['uptime'];
        document.getElementById("os-release").innerHTML = obj['os-release'];
    } catch (error) {
        console.log(error);
    }
    console.log('Message from server ', event.data);
});


setTimeout(() => {
    const obj = { hello: "world" };
    const blob = new Blob([JSON.stringify(obj, null, 2)], {
      type: "application/json",
    });
    console.log("Sending blob over websocket");
    socket.send(blob);
}, 1000);

setTimeout(() => {
    socket.send('About done here...');
    console.log("Sending close over websocket");
    socket.close(3000, "Crash and Burn!");
}, 3000);

