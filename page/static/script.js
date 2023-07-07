(async function getData() {
    const res = await fetch("api");
  
    try {
      const obj = await res.json();
      console.log(obj);
      const divElement = document.getElementById('jsonDiv');
      let htmlContent = '';
      obj.forEach(item => {
        const uptime = parseSeconds(item.uptime);
        htmlContent += `<p>Alias: ${item.alias}, ID: ${item.id}, OS Version: ${item.os_release}, Uptime: ${uptime.hours}:${uptime.minutes}:${uptime.seconds}</p>`;
      });
      divElement.innerHTML = htmlContent;
    } catch (error) {
      console.log(error);
    }
  })();
  
  function parseSeconds(inp) {
    const hours = Math.floor(inp / 3600);
    const minutes = Math.floor((inp % 3600) / 60);
    let seconds = Math.floor((inp % 3600) % 60);
    seconds = seconds < 10 ? '0' + seconds : seconds;
    return { hours, minutes, seconds };
  }
  