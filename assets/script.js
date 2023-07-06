(async function getData() {
    const res = await fetch("api");
  
    try {
      const obj = await res.json();
      console.log(obj);
      const divElement = document.getElementById('jsonDiv');
      let htmlContent = '';
      obj.forEach(item => {
        const uptime = parseSeconds(item.uptime);
        htmlContent += `<p>Alias: ${item.alias}, ID: ${item.id}, OS Version: ${item.os_release}, Uptime: ${uptime.hours} hours ${uptime.minutes} minutes</p>`;
      });
      divElement.innerHTML = htmlContent;
    } catch (error) {
      console.log(error);
    }
  })();
  
  function parseSeconds(seconds) {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return { hours, minutes };
  }
  