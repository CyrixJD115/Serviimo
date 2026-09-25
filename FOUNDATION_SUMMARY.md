# Serviimo Foundation

A personal-use server foundation for studying and reimplementing Aniimo's server-side protocol. This is an architecture and protocol milestone, not a complete game server.

## What is implemented

- A single-process Rust runner starts the TCP login gate, CDN HTTP server, and office HTTP stubs. Ctrl-C shuts the listeners down.
- The login gate handles the plaintext token handshake, RSA-OAEP-SHA1 key exchange, and encrypted Method 11 acknowledgment.
- After the handshake, each direction keeps a continuous RC4 keystream and zlib stream. The gate routes the expected LoginService.verifySdk request and returns a local token with a stub role list.
- The CDN generates server_list_publish.json using Apiimo's ServerListPublish types. Its msGateAddrList points to the configured Serviimo gate.
- The office server returns stub JSON for /api/init and /profile-sessions/start.
- SQLite tables and upsert methods cover accounts and opaque world-state snapshots.
- World-gate connectServer and RoleService.CS_Login types and a route entry are present. There is no world-gate behavior or simulation yet.
- Shared JSON configuration, command-line overrides, first-run RSA key generation, and AniiSignal config-file generation are included.

The private key is stored locally and is not included in this summary. The server prints the public key so a future client key-injection path can use it.

## Verification

The requested checks passed:

- cargo build --release
- cargo test --workspace

The tests cover the frame layout and a client-shaped handshake: Method 1, Method 19 and Method 10, RSA-OAEP-SHA1 Method 2, encrypted Method 11, and encrypted RPC frames using continuous RC4/zlib state.

A local startup and curl smoke check also passed. The generated CDN response included:

    "msGateAddrList": [
      "127.0.0.1:32100"
    ]

The office endpoints returned stub JSON. These checks confirm the local implementation runs; they do not prove that the game client can log in.

## Known protocol-source conflicts

The checked-out Apiimo revision differs from parts of the supplied specification:

- The referenced apiimo-network/src/session.rs does not exist in that checkout. Its current client code has plain framing and one-shot RC4 helpers, but no continuous session codec.
- Apiimo's current RSA client defaults to PKCS#1 v1.5. It uses OAEP-SHA1 only when APIIMO_RSA=oaep-sha1 is set.
- OAEP performs its own padding. Zero-padding a plaintext to the RSA modulus length before OAEP is not valid; Serviimo passes the protobuf directly to OAEP.
- Apiimo's Rust handshake types disagree with its vendored client_gate.proto fields for EncryptKey and EncryptKeyString.
- The method 19 payload is not settled. Apiimo's probe comment describes two opaque 12-byte fields, while common.proto defines a different GateInfo layout.
- The supplied dump has office profiling types but does not contain the gate/RPC identifiers needed to confirm these wire details.

The findings and implementation notes are in PLAN.md. Apiimo source and AniiSignal source were left unchanged.

## Deliberately out of scope

This foundation does not implement real ticket validation, TLS, XDF repacking, AniiSignal hooks, world-gate behavior, ECS simulation, or a complete office API contract. No real game-client login has been demonstrated.

## Next steps

1. Verify the RSA mode and method 19 payload against the selected client build.
2. Complete a client public-key injection path and test one real verifySdk exchange.
3. Implement world-gate connectServer and CS_Login behavior.
4. Connect account and world-session state to SQLite.

## Source map

- src/main.rs: service startup and Ctrl-C handling
- src/config.rs: shared configuration and CLI overrides
- serviimo-gate/: framing, handshake, continuous transport, and verifySdk stub
- serviimo-cdn/: generated server list and static content
- serviimo-office/: HTTP stubs
- serviimo-store/: SQLite schema and upserts
- serviimo-world/: world protocol types and route entry
- PLAN.md: detailed protocol findings and roadmap

