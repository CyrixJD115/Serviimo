# Serviimo

A personal-use Aniimo server foundation. The single-process binary starts a
TCP login gate, a CDN HTTP server, and office HTTP stubs. It also initializes
a SQLite store and writes an AniiSignal configuration containing the gate
public key.

The game login flow is not end-to-end yet. The world gate has types and a
router entry only; there is no world listener, XDF writer, or ECS simulation.

## Run

From this directory, run: cargo run --release

The defaults bind the login gate to 127.0.0.1:10000, the CDN to
127.0.0.1:18080, and the office API to 127.0.0.1:18081. The generated
server list is available at /server_list_publish.json. Press Ctrl-C to stop.

A JSON config file can set all service bind addresses, the published gate
address, RSA key path, SQLite path, CDN document root, and AniiSignal config
path. Command-line options override those values; use --help for the list.

See PLAN.md for protocol assumptions, limitations, and next steps.
