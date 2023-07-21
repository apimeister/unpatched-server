import { ref, watchEffect } from 'vue'

export default {
    mounted () {
        M.AutoInit()
    },
    setup() {
        const agents = ref(null);
        watchEffect(async () => { 
            agents.value = await (await fetch('/api/v1/agents')).json();
        })
        const agent_detail = ref(null);
        let currentId = ref(null)
        watchEffect(async () => { 
            if (currentId.value) {
                agent_detail.value = await (await fetch(`/api/v1/agents/${currentId.value}`)).json();
            }
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
            agent_detail,
            currentId,
            parseUptime,
            bytesToSize
        }
    },
    
    template: /*html*/`
    <ul class="collapsible">
        <li v-for="agent in agents" :id="agent.id" @click="currentId = agent.id">
            <div class="collapsible-header">
                <i class="material-icons">fingerprint</i>
                <span>{{ agent.alias }}</span>
                <span>Uptime: {{ parseUptime(agent.uptime) }}</span>
                <span>OS Version: {{ agent.os_release }}</span>
                <span class="new badge" data-badge-caption="units">{{ agent.units.length }}</span>
            </div>
            <div class="collapsible-body">
                <p>Alias: {{ agent.alias }}</p>
                <p>ID: {{ agent.id }}</p>
                <p>OS Version: {{ agent.os_release }}</p>
                <p>Uptime: {{ parseUptime(agent.uptime) }}</p>
                <p v-if="agent_detail">Memory: used {{ bytesToSize(agent_detail.memory.used_mem) }} | free {{ bytesToSize(agent_detail.memory.free_mem) }} | available {{ bytesToSize(agent_detail.memory.av_mem) }} | total {{ bytesToSize(agent_detail.memory.total_mem) }}</p>
                <p v-if="agent_detail" >Units:</p>
                <p v-if="agent_detail" v-for="unit in agent_detail.units">{{unit.name}}</p>
            </div>
        </li>
    </ul>`
}
