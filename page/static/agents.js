import { ref, watchEffect } from 'vue'

export default {
    mounted () {
        M.AutoInit()
    },
    setup() {
        const API_URL = '/api';
        const agents = ref(null);
        watchEffect(async () => {
            const url = `${API_URL}`;
            agents.value = await (await fetch(url)).json();
          })

        function parseUptime(inp) {
            const hours = Math.floor(inp / 3600);
            const minutes = Math.floor((inp % 3600) / 60);
            let seconds = Math.floor((inp % 3600) % 60);
            seconds = seconds < 10 ? '0' + seconds : seconds;
            const readable_time = /*html*/`${hours}:${minutes}:${seconds}`;
            return readable_time;
        }

        function bytesToSize(bytes) {
            const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB']
            if (bytes === 0) return 'n/a'
            const i = parseInt(Math.floor(Math.log(bytes) / Math.log(1024)), 10)
            if (i === 0) return `${bytes} ${sizes[i]}`
            return `${(bytes / (1024 ** i)).toFixed(1)} ${sizes[i]}`
          }

        return {
            agents,
            parseUptime,
            bytesToSize
        }
    },
    
    template: /*html*/`
    <ul class="collapsible">
        <li v-for="agent in agents">
            <div class="collapsible-header">
                <i class="material-icons">fingerprint</i>
                {{ agent.alias }}
                <span>Uptime: {{ parseUptime(agent.uptime) }}</span>
                <span class="new badge" data-badge-caption="units">{{ agent.units.length }}</span>
            </div>
            <div class="collapsible-body">
                <p>Alias: {{ agent.alias }}</p>
                <p>ID: {{ agent.id }}</p>
                <p>OS Version: {{ agent.os_release }}</p>
                <p>Uptime: {{ parseUptime(agent.uptime) }}</p>
                <p>Memory: used {{ bytesToSize(agent.memory.used_mem) }} | free {{ bytesToSize(agent.memory.free_mem) }} | available {{ bytesToSize(agent.memory.av_mem) }} | total {{ bytesToSize(agent.memory.total_mem) }}</p>
            </div>
        </li>
    </ul>`
}
