# Monitoring Server

[![codecov](https://codecov.io/gh/apimeister/unpatched-server/branch/main/graph/badge.svg?token=WEVL9G0F3F)](https://codecov.io/gh/apimeister/unpatched-server)

## usage

```shell
cargo run [-- [--bind localhost] [--port 3000]]
```

## usage details

1. start server
2. start [agent](https://github.com/apimeister/monitor-agent) to send data to server
3. open webgui at server:port - example 127.0.0.1:3000
4. refresh to change data
