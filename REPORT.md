# Serviimo - a personal-use Aniimo server, feasibility and plan

Date: 2026-09-22
Status: skeleton only (this directory has stub crates + this report)
Companion reports:
  ../.reports/keyhunt-and-save-pull-2026-09-22.md  (protocol + gaps)
  ../PROJECT-REPORT.md                            (game-wide RE)

## 0. Verdict

Possible, and this game is a better private-server candidate than most.
Four facts proven during the Apiimo work make it tractable:

1. Server addressing is DATA: the client reads server_list_publish.json
   from the CDN and dials whatever msGateAddrList contains. Redirect one
   URL and the client comes to us - no binary patching.
2. We hold both ends of the crypto on our own server: the gate RSA
   keypair is ours, and the session cipher is RC4 keyed with
   SHA1(16-byte seed) that the client encrypts to OUR public key.
   Everything the client sends is plaintext to us. The key-hunt problem
   from the live server simply does not exist here.
3. The protocol schemas ship inside the client: all .proto files live in
   LuaScripts.xdf (already extracted in Apiimo), and the world
   property-sync data model ships as CompactPropertySchema /
   ProtobufConst in the same bundle.
4. IL2CPP Unity 2022.3.62f1 with a complete dump (dump.cs) - any
   MelonLoader/BepInEx mod has full type information.

## 1. What the real server side does (mapped from RE)

```
CDN   server_list_publish.json .......... which gates exist (JSON)
HTTPS worldx-office-api /api/init, /profile-sessions/start,
      FPX/pxauth ticket auth, notices ... account bootstrap (JSON)
TCP   ms-gate :10000 .................... requestEncryptToken(1) ->
                                        responseEncryptToken(10),
                                        confirmEncryptKey(2),
                                        connectServer(3), verifySdk,
                                        CS_Login, heartbeat(6),
                                        microservice(7/8, msgpack)
TCP   world gate :20000 ................. ECS entity/property sync
                                        (this stream IS the save)
```

Framing everywhere: [u32 LE total][u16 LE method][protobuf body].
Session cipher: RC4. Key material: 20-byte SHA1 of a client-generated
16-byte seed (proven in xlua.dll; implemented in Apiimo
apiimo-network/src/client.rs).

## 2. Crate layout (this skeleton)

| crate            | role                                           |
|------------------|------------------------------------------------|
| serviimo (bin)   | composes everything, one process to start      |
| serviimo-cdn     | server list + pinned Lua/patch content         |
| serviimo-office  | HTTP account/profile/ticket stubs              |
| serviimo-gate    | gate protocol server (the Apiimo twin)         |
| serviimo-world   | ECS world simulation                           |
| serviimo-store   | SQLite persistence (accounts + world saves)    |

Reuse from Apiimo (enable the path dependency when ready):
apiimo-network gives framing, PbMessage protobuf codec, rc4, the exact
handshake material, ServerListPublish serde types, identity types, and
the vendored .proto files. serviimo-gate is essentially the listener
half of GateConnection.

## 3. Tiered roadmap

Tier 1 - "online" (days)
  [ ] gate server answers requestEncryptToken with a token
  [ ] confirmEncryptKey accepted (our RSA keypair; decrypt with private
      key, recover the 20-byte RC4 key - protocol fully transparent)
  [ ] connectServer + connectResponse accepted
  [ ] CDN serves a server list pointing at our gate; client redirected
      via /etc/hosts + local HTTPS with a CA added to the Proton prefix
  [ ] client reaches an empty world without crashing
Success test: the game's own login screen completes against serviimo.

Tier 2 - persistence + basic world (weeks to a couple of months)
  [ ] decode-and-log every post-handshake frame (we have the RC4 key)
  [ ] property-sync schema extraction from the client Lua
  [ ] SQLite accounts + world-part saves
  [ ] static entity spawns; two clients see each other
  [ ] MelonLoader mod (optional): config URL override + HTPSec stub

Tier 3 - gameplay (the long haul)
  [ ] server-side spawns/AI/combat/economy
  [ ] chat, matchmaking-style features
This tier is reimplementing the live game's simulation; treat it as
open-ended.

## 4. Gotchas, known up front

- Anti-cheat: NetEase HTPSec (NTESRiskSecProtect) is a client sensor;
  its server-side validation is ours on a private server, but the SDK
  still phones home on the client. The MelonLoader mod can stub
  getToken/dataSign. HTPSec may detect injection - stub it early.
- Content version: the client hot-downloads LuaScripts updates; the CDN
  crate must pin the client to one content version (serve the XDFs we
  already extracted) or protocol mismatches will bite.
- HTTPS: office API + CDN are HTTPS. Either install a local CA into the
  Proton wine prefix, or hook the URL/cert paths with the mod. Watch
  for cert pinning when we get there.
- Steam auth: FPXSteam wraps platform identity; the office stub can
  accept anything at first, then emulate the ticket chain (the ticket
  format is already decoded in apiimo-network::identity).
- KCP: the Lua mentions KCP as the primary transport with a TCP
  fallback ("kcp is forbidden, switch to tcp"). TCP-only is fine; KCP
  is a later optimization.
- Legal/personal-use line: FunPlus ToS applies to the live game; keep
  serviimo personal, do not distribute their assets, do not profit.

## 5. Immediate next steps (when we choose to start)

1. Flip the apiimo-network path dependency on in Cargo.toml.
2. serviimo-gate: listener + method 1 -> 10 reply + method 2 acceptance
   (decrypt keyString with our private key, log SHA1(seed)).
3. serviimo-cdn: serve a hand-written server_list_publish.json over
   plain HTTP first (test whether the client accepts http for the CDN;
   the isSSL flag exists per endpoint in the list format).
4. Point the client at it (hosts entry), watch the first real client
   handshake arrive, and iterate on the login sequence using the
   client's own error handling as the guide.

Skeleton created 2026-09-22; nothing here builds a server yet - the
stubs compile and carry the plan.
