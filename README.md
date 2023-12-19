# http-mtls-proxy

An mTLS proxy (http to mTLS-https) written in Rust, meant to run in the
Terminal. For Spring Boot Test Containers, have a look at
[http-mtls-proxy-boot3-test-container](https://github.com/TelenorNorway/http-mtls-proxy-boot3-test-container).

## Installation

```sh
cargo install http-mtls-proxy
```

## Usage

```
Usage: http-mtls-proxy [OPTIONS] <MAPPING1> [MAPPING]...

Arguments:
  <MAPPING1>    Define the request mappings. Ex ':9000/foo/:path*=https://example.com/{path}'
  [MAPPING]...

Options:
      --client <CLIENT>  Create a client for outbound requests. Ex. --client foo=*.example.com
      --cert <CERT>      Define the certificate a client should use for mTLS. Ex. --cert foo=/path/to/example.com.pem
      --key <KEY>        Define the key a client should use for mTLS. Ex. --key foo=/path/to/example.com.key
  -h, --help             Print help
  -V, --version          Print version
```

Example:

```sh
http-mtls-proxy \
  --client "aZone=https://*.mtls.a-zone.internal:*/*" \
  --cert "aZone=/path/to/a-zone-user123.pem" \
  --key "aZone=/path/to/a-zone-user123.key" \
  --client "bZone=https://*.mtls.b-zone.internal:*/*" \
  --cert "bZone=/path/to/b-zone-user123.pem" \
  --key "bZone=/path/to/b-zone-user123.key" \
  ":3000/:letter([ab])-zone/:service/:path*=https://{service}.mtls.{letter}-zone.internal{/path}" \
  ":4000/something-special/:path*=https://something-special.mtls.b-zone.internal{/path}"
```
