(async function getData() {
    const res = await fetch("api");
  
    try {
      const obj = await res.json();
      console.table(obj);
      const divElement = document.getElementById('jsonDiv');
      let htmlContent = '';
      obj.forEach(item => {
        const uptime = parseSeconds(item.uptime);
        const free_mem = parseMem(item.memory.free_mem);
        const av_mem = parseMem(item.memory.av_mem);
        const used_mem = parseMem(item.memory.used_mem);
        const total_mem = parseMem(item.memory.total_mem);
        htmlContent += `<p>
        Alias: ${item.alias}, 
        ID: ${item.id}, 
        OS Version: ${item.os_release}</p>
        <p>Uptime: ${uptime.hours}:${uptime.minutes}:${uptime.seconds}</p>
        <p>Memory (GB): used ${used_mem.gb} | free ${free_mem.gb} | available ${av_mem.gb} | total ${total_mem.gb}</p>
        <p>Memory (MB): used ${used_mem.mb} | free ${free_mem.mb} | available ${av_mem.mb} | total ${total_mem.mb}</p>
        <p>Memory (KB): used ${used_mem.kb} | free ${free_mem.kb} | available ${av_mem.kb} | total ${total_mem.kb}</p>
        </p>`;
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

  function parseMem(i) {
    const kb = Math.floor(i / 1024);
    const mb = Math.floor(kb / 1024);
    const gb = Math.floor(mb / 1024);
    return { kb, mb, gb }
  }
  