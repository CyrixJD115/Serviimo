# Serviimo - a personal-use Aniimo server: feasibility and plan

Date: 2026-09-25 (rewritten; supersedes the 2026-09-22 draft that was called REPORT.md)
Status: foundation implemented; local services compile, but the game flow is not end-to-end.
Companion reports, all in the parent workspace:
  ../.reports/PROJECT-REPORT.md                        game-wide RE, anti-cheat, endpoints
  ../.reports/REPORT.md                                 the full Aniimo RE report
  ../.reports/keyhunt-and-save-pull-2026-09-22.md      the original gap analysis
  ../.reports/rsa-key-file-map.md                      where the gate key does/does not live
  ../PLAN.md                                            the live-server hunt, current state
  ../Apiimo_src/                                        the protocol toolkit (github CyrixJD115/Apiimo)

## 0. What changed since the 2026-09-22 draft

This is the important section, because the old draft's Tier 1 plan was
written against a protocol model that has since been corrected three
times. Two of the corrections invalidate work items outright.

1. The session key is OURS, trivially. The draft treated "recover the
   RC4 key" as a crypto problem. It is not: we generate the key, we
   hold the private half of the RSA pair, we hand the client the public
   key. There is no hunt.
2. `connectServer` is a WORLD-gate message, not an ms-gate message. The
   draft's Tier 1 listed "connectServer + connectResponse accepted" as
   a login-gate milestone. The real client never sends it to the
   ms-gate. The production login gates are literally named
   `ms-gate-CLOSE` and answer `ConnectServerResp{type:0}` (which the
   protocol calls Busy) **by design**. The ms-gate flow is exactly two
   steps: handshake, then `verifySdk`. This single fact is what burned
   ~50 replay variants in the live hunt - see ../PLAN.md.
3. Post-handshake transport is continuous, not per-frame. The draft
   (and an intermediate Apiimo note) described RC4 per dispatch with a
   fresh KSA each frame. The live-verified model is one continuous RC4
   keystream per direction wrapping one continuous zlib stream per
   direction, sync-flushed per message, with the
   `[u32 len][u16 method][protobuf]` frames living *inside* the
   inflated chunk. The checked-out Apiimo revision does not contain the
   referenced session.rs; its current client.rs only models plain framing.
   See section 9 for the source findings.
4. The save stream is not protobuf either. It is phonestcore's
   `PropertySync` `LuaLazyDecoder` - schema-bundled lazy MessagePack
   (`mp_lazy_encode`/`decode_client`, `DeclareBundle`), symbols around
   `0x6e4xxx` in `xlua.dll`. The old plan assumed plain protobuf
   property sync.

What did not change: the verdict, the crate layout, the CDN pivot, and
the "we hold both ends of the crypto" argument. That argument is now
much stronger, because the shipped client literally reads our public key
out of `conf.json` in `LuaScripts.xdf`.

## 1. Verdict

Possible, and this game is a better private-server candidate than most.
Five facts proven during the Apiimo work make it tractable:

1. Server addressing is DATA: the client reads `server_list_publish.json`
   from the CDN and dials whatever `msGateAddrList` contains. Redirect
   one URL and the client comes to us - no binary patching.
2. We hold both ends of the crypto, and this is now proven end to end
   rather than argued. The gate public key is the `pubKeyData` PEM
   inside `xfs/luascripts/Network/conf.json` in the client's own
   `LuaScripts.xdf` - a plain, readable ZIP entry. The wrap is
   RSA-OAEP-SHA1, the session cipher is RC4, and the session key is a
   client-generated 20-byte SHA-1 that the client encrypts to whatever
   public key we put in that file. Everything the client sends is
   plaintext to us by construction.
3. The protocol schemas ship inside the client: all 7 `.proto` files
   live in `LuaScripts.xdf` (vendored in Apiimo under
   `apiimo-network/proto/`), and the world property-sync data model
   ships as `CompactPropertySchema` / `ProtobufConst` in the same
   bundle.
4. IL2CPP Unity 2022.3.62f1 with a complete dump (28,348 types,
   `dump.cs`) - any MelonLoader/BepInEx mod has full type information.
5. The RPC envelope is decoded down to the byte. `MsRequest` is
   `{rid, service, method, params, callerId, context, options}` with
   msgpack params, and the real first login message has been rebuilt
   byte-exactly (573 B on the wire) and pinned by a unit test.

## 2. What the real server side does (mapped from RE)

```
CDN   server_list_publish.json .......... which gates exist (JSON, public)
      LuaScripts.xdf ................... game logic + conf.json pubKeyData
HTTPS worldx-office-api /api/init, /profile-sessions/start,
      FPX/pxauth ticket auth, notices ... account bootstrap (JSON)
TCP   ms-gate :10000 .................... handshake, then verifySdk ONLY
TCP   world gate :20000 ................. connectServer, then
                                        RoleService.CS_Login, then the
                                        ECS entity/property stream
                                        (this stream IS the save)
```

Framing everywhere: `[u32 LE total][u16 LE method][protobuf body]`.

Session cipher, the corrected model: after method 2 succeeds, each
direction becomes ONE continuous RC4 keystream (KSA run once on the
20-byte key, never resynced) wrapping ONE continuous zlib stream
(sync-flushed per message, so the deflate state carries across
messages). Inflated chunks are back-to-back
`[u32 len][u16 method][protobuf]` dispatches.

Key material: 16-byte client-generated seed, SHA-1'd to 20 bytes,
RSA-OAEP-SHA1 wrapped with random head (5..19 B) and tail (13..32 B)
padding, sent as `EncryptKey{head, key, token, tail}` inside
`EncryptKeyString`, method 2.

## 3. Two ways to point the client at us

The mod and the vanilla path are not competing designs. The **wire
protocol is identical in both** - `serviimo-gate` does not know or care
which one is in use, because the mod changes only how the client
*learns* our address, never what it sends. Pick per tier; the server
side is untouched either way.

|                        | vanilla (no mod)                     | mod-assisted                          |
|------------------------|--------------------------------------|---------------------------------------|
| CDN redirect           | hosts entry + local CA in Proton     | mod overrides the config/server-list URL |
| gate public key        | repacked `LuaScripts.xdf` served by our CDN | same, or handed in-process by the mod |
| TLS                    | local CA trusted by the wine prefix  | mod can pin or bypass the check       |
| HTPSec                 | must be tolerated offline            | mod stubs `getToken` / `dataSign`     |
| patcher overwrites XDF | CDN must keep re-serving our XDF      | mod can re-apply the key after a patch |
| debuggability          | client error strings only            | full managed types, breakpoints        |

The vanilla column is the baseline and stays the acceptance test: the
goal is an unmodified client completing its own login screen against
serviimo. The mod column exists because vanilla has three genuinely
painful edges - the wine CA install, the patcher clobbering our XDF,
and HTPSec phoning home - and the mod removes all three in about a day
of work. It is an accelerator, not a dependency.

Keep the mod's surface small and boring, in this priority order:

1. **Endpoint override** - point the config/CDN URL at localhost. This
   is the single highest-value hook and unblocks the HTTPS problem
   wholesale.
2. **Config injection** - return our `conf.json` in memory with our
   `pubKeyData`, so the XDF repack stops being load-bearing.
3. **HTPSec stub** - return a dummy `AntiCheatResult` for
   `getToken`/`getDataSign`/`ioctl` on `NTESRiskSecProtect`.
4. **Optional skips** - crash reporting, analytics, and the FunPlus
   SDK bootstrap, once they prove to be the thing actually blocking.

Avoid anything that rewrites protocol messages. The moment the mod
edits a payload, the vanilla baseline stops being a valid test and we
lose the ability to tell a Serviimo bug from a mod bug.

## 4. Crate layout (this skeleton)

| crate            | role                                           |
|------------------|------------------------------------------------|
| serviimo (bin)   | composes everything, one process to start      |
| serviimo-cdn     | server list + pinned Lua/patch content         |
| serviimo-office  | HTTP account/profile/ticket stubs              |
| serviimo-gate    | gate protocol server (the Apiimo twin)         |
| serviimo-world   | ECS world simulation                           |
| serviimo-store   | SQLite persistence (accounts + world saves)    |
| serviimo-mod     | the optional MelonLoader client mod (section 3) |

Apiimo's apiimo-network path dependency is enabled in the root workspace.
It supplies the plain frame codec, PbMessage, one-shot rc4, handshake and RPC
types, ServerListPublish serde types, identity types, and vendored proto files.
The current crate exposes GateConnection as a client probe; it does not expose
LiveSession or a server-side stream codec. Serviimo implements its own server
listener and continuous session codec while reusing the published wire types.

## 5. Tiered roadmap

Tier 1 - "online" (days)
  [x] enable the apiimo-network path dependency
  [x] generate and persist an RSA keypair for the gate
  [ ] rebuild LuaScripts.xdf with our public key as
      `xfs/luascripts/Network/conf.json` -> `pubKeyData`
      (BLOCKER, see section 7: apiimo can read XDF but not write it)
  [x] serviimo-cdn serves a generated server_list_publish.json whose
      msGateAddrList points at our gate
  [x] serviimo-gate: answer method 1 with method 19 (gate info) +
      method 10 (responseEncryptToken, 64-bit token); accept method 2
      with RSA-OAEP-SHA1 and validate the recovered 20-byte RC4 key
  [x] stand up continuous RC4 + continuous zlib transport in both
      directions according to the wire behavior specified above
  [x] answer LoginService.verifySdk (MsRequest, msgpack fixarray
      params) with a stub status, Token, and role list
  [ ] point the client at our CDN (hosts entry + local CA in the
      Proton prefix) and get the game's own login screen to complete
  [ ] THEN, in parallel: the mod's endpoint-override hook, so the
      wine-CA and XDF-clobber edges stop costing time
Success test: the unmodified client logs in against serviimo.

Tier 2 - world entry + persistence (weeks)
  [x] world-gate request types and RoleService.CS_Login router entry;
      world-gate behavior and simulation remain unimplemented
  [ ] decode-and-log every post-login frame - we hold the key, so
      every entity/property message is readable
  [ ] property-sync schema extraction from the client Lua
      (sendPropertyDeclare carries the schema itself)
  [x] SQLite account and world-state schema with upsert methods
  [ ] static entity spawns; two clients see each other
  [ ] mod: HTPSec stub, so an injected client is not self-reporting

Tier 3 - gameplay (the long haul)
  [ ] server-side spawns / AI / combat / economy
  [ ] chat, matchmaking-style features
This tier is reimplementing the live game's simulation; treat it as
open-ended.

## 6. Gotchas, known up front

- Anti-cheat: NetEase HTPSec (NTESRiskSecProtect) is a client sensor
  whose *server-side* validation is ours on a private server, so the
  token check itself is not a wall. The SDK still phones home from the
  client, and it may detect injection - which the mod is. Stub
  `getToken`/`dataSign` early in the mod.
- Content version: the client hot-downloads Lua updates and will
  overwrite a hand-patched `LuaScripts.xdf` on the next patch run. The
  CDN crate must serve the XDFs we extracted (with our key baked in) so
  the patcher keeps re-installing ours, or serve a version list that
  pins the client where it is. The mod's config-injection hook is the
  cleaner fix.
- HTTPS: office API and CDN are both HTTPS. Either install a local CA
  into the Proton wine prefix or hook the URL/cert paths with the mod.
  Watch for cert pinning once we get that far.
- Steam auth: FPXSteam wraps platform identity. The office stub can
  accept anything at first, then emulate the ticket chain - the ticket
  format is decoded in `apiimo-network::identity`. Nothing here
  touches the real FunPlus auth servers.
- Transport: the client prefers KCP with a `ws://` TCP fallback
  ("kcp is forbidden, switch to tcp"). The Apiimo captures and replay
  both used plain TCP successfully, so start TCP-only; KCP is a later
  optimization if the client insists.
- zlib level matters: captures show the client compressing at level 0
  (stored). The checked-out Apiimo revision has no session codec to copy.
  Serviimo keeps one zlib encoder and decoder per direction, with level-0
  compression and one flush per frame. Local tests cover state continuity;
  a real-client capture must confirm the exact flush output.
- Mod discipline: a mod that changes *where* the client connects is
  fine; one that changes *what* it sends will make every later bug
  ambiguous. Keep payload edits out (section 3).
- Legal/personal-use line: FunPlus ToS applies to the live game. Keep
  serviimo personal, do not redistribute their assets, do not profit.

## 7. The one real blocker: writing XDF

The vanilla path in Tier 1 hinges on substituting our own public key
into `LuaScripts.xdf`, and today nothing in the workspace can do that.

`apiimo-network/src/lua.rs` has `XdfContainer` with `open`,
`from_bytes`, `read`, `find`, `entries` - all read paths. There is no
writer, and `apiimo xdf-cat` writes one entry to stdout rather than
back into a container.

The good news is that the format is not in the way: `XdfContainer` is a
plain ZIP (`zip::ZipArchive`), the shipping pack uses stored
(uncompressed) entries with no encryption, and the client's own ZipFile
layer rejects AES entries outright. So a repack is a `zip::ZipWriter`
loop - copy every entry, substitute the one `conf.json`. Roughly 30
lines in `lua.rs`, which is also where it belongs.

Until that exists, the vanilla path cannot start, and no amount of gate
work will help - the client will keep encrypting to FunPlus's key. The
mod's config-injection hook is the parallel escape hatch, and honestly
the better long-term answer, but the vanilla repack is the thing that
proves the CDN pivot works at all.

## 8. Next steps after the foundation

1. Verify the method 19 payload and RSA mode against the chosen client build.
   The current Apiimo client needs APIIMO_RSA=oaep-sha1 to exercise OAEP-SHA1.
2. Choose and complete the public-key delivery path: a future runtime
   AniiSignal injection or an XDF repack. The foundation only emits the mod
   config and does not implement the mod or edit Apiimo.
3. Test one real client handshake and verifySdk request. The local tests prove
   the implemented framing and crypto path, not compatibility with the game.
4. Implement the world-gate connectServer and RoleService.CS_Login behavior.
5. Connect the verified account/session to SQLite and add world-state updates.
6. Add TLS and match the office response contracts when the client requires it.

Skeleton created 2026-09-22; rewritten 2026-09-25 after the live
protocol work settled the transport model, the ms-gate/world-gate
split, and the lazy-msgpack save encoding. Foundation services now run,
but the game login flow is not end-to-end.


## 9. Foundation implementation notes and source findings

The foundation adds a single-process runner, shared JSON configuration and CLI
overrides, a TCP ms-gate, generated CDN server-list JSON, HTTP office stubs,
SQLite account/world-state tables, and world-gate types. The world gate has no
listener or ECS behavior. The gate returns a random local token and stub role
metadata; it does not validate a production ticket.

The gate key is stored as PKCS#1 PEM with mode 0600 on Unix. Only its public
PEM is printed and written into the generated AniiSignal JSON. The mod source
folder remains comments-only; the JSON is runtime configuration, not mod code.

The checked-out Apiimo source differs from the protocol instructions in these
ways:

- The referenced apiimo-network/src/session.rs is absent. The current
  client.rs has plain TCP framing and one-shot RC4 helpers, but no continuous
  RC4 stream state or zlib stream codec. Serviimo implements the specified
  continuous streams in its gate crate.
- client.rs SessionKeyMaterial::rsa_wrap defaults to PKCS#1 v1.5. OAEP-SHA1
  is available only when APIIMO_RSA=oaep-sha1; the Serviimo gate follows the
  requested OAEP-SHA1 format.
- OAEP encodes its own padding and cannot accept a plaintext zero-padded to the
  RSA modulus length. Apiimo's OAEP branch encrypts the protobuf directly.
  Serviimo likewise sends the unpadded protobuf to OAEP.
- The checked-in client_gate.proto declares only EncryptKey.key field 2 and
  token field 3, and declares EncryptKeyString empty. The Rust types in
  handshake.rs instead encode head_padding field 1, key field 2, token field 3,
  tail_padding field 4, and key_string field 1. The server follows the Rust
  types because they match the requested client payload shape.
- Apiimo's method 19 probe comment records two opaque 12-byte values. Its
  common.proto GateInfo instead defines fields 1, 3, 4, 5, 6, and 7, so the
  probe summary does not establish that method 19 is that GateInfo message.
  The initial server push uses two 12-byte fields as placeholders.
- The supplied dump.cs contains LoadSessionKey and office profiling DTOs such
  as StartProfileSessionRequest and StartProfileSessionResponse, but searches
  found no GateClientProtobufHandler, verifySdk, MsRequest,
  responseEncryptToken, or confirmEncryptKeyAck identifiers. It does not
  independently prove the RPC method IDs, protobuf fields, method 19 payload,
  or zlib flush behavior; those details come from Apiimo and the supplied
  protocol notes, not this dump.

A successful local build or CDN curl check does not prove that the live client
can log in. Follow the steps in section 8 to verify the protocol against the
chosen client build before implementing world-gate behavior.
