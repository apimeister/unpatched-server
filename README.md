# Unpatched Server

[![codecov](https://codecov.io/gh/apimeister/unpatched-server/branch/main/graph/badge.svg?token=WEVL9G0F3F)](https://codecov.io/gh/apimeister/unpatched-server)

## usage

```shell
A bash first fleet management solution for VMs and physical hosts

Usage: unpatched-server [OPTIONS]

Options:
  -b, --bind <BIND>           bind adress for frontend and agent websockets [default: 127.0.0.1]
  -p, --port <PORT>           bind port for frontend and agent websockets [default: 3000]
      --no-tls                deactivate tls
      --auto-accept-agents    auto-accept new agents
      --seven-part-cron       use 7 part instead of 5 part cron pattern
      --cert-folder <FOLDER>  Sets the certificate folder [default: ./self-signed-certs]
  -h, --help                  Print help
  -V, --version               Print version
```

### usage details

1. start server
2. open webgui at server:port - example `127.0.0.1:3000`
3. start [agent](https://github.com/apimeister/monitor-agent) to send data to server
   - look into server log
   - copy out agent id
   - go to `https://your-server.x/api` -> hosts -> approval
   - paste id and execute, your agent is now allowed to send data
4. refresh to change data

## TLS

By default this server expects an `unpatched.server.key` and `unpatched.server.crt` file under `./self-signed-certs`. To change this behavior set a new path with the `--cert-folder` option. The file names are not changable.

### Web Certificates

Add your key-pair as `unpatched.server.key` and `unpatched.server.crt` to the cert-folder

### Self-signed certificate example

1. Make a new folder `./self-signed-certs` and cd into it
2. Generate an internal rootCA pair and leaf Cert
3. copy `rootCA.crt` to agent host and follow instructions in [agent repo](https://github.com/apimeister/monitor-agent)

```shell
# with openssl
server_dns="127.0.0.1"; # url dns name

# make a new file called v3.ext and add 

basicConstraints        = CA:FALSE
keyUsage                = digitalSignature,dataEncipherment
extendedKeyUsage        = clientAuth,serverAuth
subjectAltName          = @alt_names

[alt_names]
DNS.1 = localhost
IP.2 = 127.0.0.1
IP.3 = ::1

# create root-ca
openssl req -x509 -newkey rsa:4096 -nodes -out rootCA.crt -keyout rootCA.key -days 365 -subj "/O=internal/CN=$server_dns";

# create key and signing request
openssl genrsa -out unpatched.server.key 4096;
openssl req -new -sha256 -key unpatched.server.key -subj "/O=internal/CN=$server_dns" -out unpatched.server.csr -addext subjectAltName=DNS:$server_dns;
# check request
openssl req -in unpatched.server.csr -noout -text;
# if no v3.ext is added make sure to use another way to make the crt file x509 v3, otherwise a certVersion error will occur
openssl x509 -req -in unpatched.server.csr -CA rootCA.crt -CAkey rootCA.key -CAcreateserial -out unpatched.server.crt -days 500 -sha256 -extfile v3.ext;
# check leaf certificate
openssl x509 -in unpatched.server.crt -text -noout;
```
