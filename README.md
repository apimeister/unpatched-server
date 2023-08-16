# Monitoring Server

[![codecov](https://codecov.io/gh/apimeister/unpatched-server/branch/main/graph/badge.svg?token=WEVL9G0F3F)](https://codecov.io/gh/apimeister/unpatched-server)

## usage

```shell
A bash first monitoring solution

Usage: cargo run [-- [--option]]

Options:
  -b, --bind <BIND>           bind adress for frontend and agent websockets [default: 127.0.0.1]
  -p, --port <PORT>           bind port for frontend and agent websockets [default: 3000]
      --no-tls                deactivate tls for frontend
      --cert-folder <FOLDER>  Sets the certificate folder [default: ./self-signed-certs]
  -h, --help                  Print help
  -V, --version               Print version
```

## usage details

1. start server
2. start [agent](https://github.com/apimeister/monitor-agent) to send data to server
3. open webgui at server:port - example 127.0.0.1:3000
4. refresh to change data

## TLS

By default this server expects a key.pem and cert.pem file under "./self-signed-certs". To change this behavior set a new path with the "--cert-folder" option. The file names and formats are not changable.

### Self-signed certificate

Generate an internal certificate pair

```shell
# with openssl
server_dns="127.0.0.1"; # url dns name
openssl req -x509 -newkey rsa:4096 -nodes -out cert.pem -keyout key.pem -days 365 -subj "/C=DE/O=internal/OU=Domain Control Validated/CN=$server_dns";
```
