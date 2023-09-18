# Unpatched Server

[![codecov](https://codecov.io/gh/apimeister/unpatched-server/branch/main/graph/badge.svg?token=WEVL9G0F3F)](https://codecov.io/gh/apimeister/unpatched-server)

## usage

```shell
A bash first monitoring solution

Usage: unpatched-server [OPTIONS]

Options:
  -b, --bind <BIND>                    bind adress for frontend and agent websockets, v6 example [::1] [default: 127.0.0.1]
  -p, --port <PORT>                    bind port for frontend and agent websockets [default: 3000]
      --no-tls                         deactivate tls
      --seven-part-cron                use 7 part instead of 5 part cron pattern
      --cert-folder <FOLDER>           Sets the certificate folder [default: ./self-signed-certs]
      --init-user <INIT_USER>          Email of first user to initialize the server with
      --init-password <INIT_PASSWORD>  Password of first user to initialize the server with
  -h, --help                           Print help
  -V, --version                        Print version
```

### usage details

1. Pre Steps:
    - use `--init-user` and `--init-password` to generate an admin user to login with (needs to be done only once)
2. start server
3. open webgui at server:port - example `127.0.0.1:3000`
4. go to agents tab and create a new [agent](https://github.com/apimeister/monitor-agent)
   - configure command shown and start agent
5. refresh to show updated data

## Authentication

- use init user to create additional users
- user must be an email address
- ip will be blacklisted after 5 wrong logins for 5 minutes
- unblock IPs inside timeframe via `/api/v1/unbock/:id` API

## hardcoded defaults

| Name | Value | Explaination
:--- | :--- | :---
| UPDATE_RATE | 5 seconds | Rate with which the server will run any process (send new scripts, update timings etc.)
| SQLITE_DB | unpatched_server_internal.sqlite | SQLite Databasefile name
| TLS_CERT | unpatched.server.crt | TLS Certificate certificate part
| TLS_KEY | unpatched.server.key | TLS Certificate key part
| JWT_SECRET | jwt.secret | File name for persisting JWT secret on disc
| API_KEY_LOGIN_TTL | 30 days | Time to go by from last checkin until an API_KEY is no longer seen as valid

## TLS

By default this server expects an `unpatched.server.key` and `unpatched.server.crt` file under `./self-signed-certs`. To change this behavior set a new path with the `--cert-folder` option. The file names are not changable.

### Web Certificates

Add your key-pair as `unpatched.server.key` and `unpatched.server.crt` to the cert-folder

### Self-signed certificate example

1. Make a new folder `./self-signed-certs` and cd into it
2. Generate an internal rootCA pair and leaf Cert - change IP if not using localhost!
3. copy `rootCA.crt` to agent host and follow instructions in [agent repo](https://github.com/apimeister/monitor-agent)

```shell
# make a new file called v3.ext and add 

basicConstraints        = CA:FALSE
keyUsage                = digitalSignature,dataEncipherment
extendedKeyUsage        = clientAuth,serverAuth
subjectAltName          = @alt_names

[alt_names] # if using another IP or DNS, change this!
DNS.1 = localhost
IP.2 = 127.0.0.1
IP.3 = ::1
```

```shell
# change this to the real IP/DNS
server_dns="127.0.0.1";

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
