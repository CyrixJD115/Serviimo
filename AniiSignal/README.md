# AniiSignal

The client half of Serviimo: a MelonLoader mod whose only job is to make an
unmodified Aniimo client connect to a personal-use server instead of FunPlus.

Design rules, from `../PLAN.md` section 3:

- The mod changes **where** the client connects, never **what** it sends. No
  payload is rewritten anywhere in this project.
- The vanilla path (no mod) stays the acceptance test for the server.
- Every hook degrades to a log line instead of throwing during load, so a game
  update that moves a target costs a feature rather than the whole session.

## Build and stage

```bash
tools/build-mod.sh              # fetch MelonLoader refs, build, self-check, stage
tools/build-mod.sh --no-stage   # build only
GAME_DIR=/path/to/Aniimo tools/build-mod.sh
```

The script fetches the MelonLoader 0.6.6 `net6` reference assemblies into
`lib/` on first run (gitignored - they are third-party binaries), compiles in
Release, runs the self-check, and copies `AniiSignal.dll` into
`$GAME_DIR/Mods/`.

MelonLoader itself is **not** installed by this script. Aniimo is IL2CPP, so
it needs the `net6` build of MelonLoader in the game directory; installing it
writes into the Steam install, which should be a deliberate act.

## Self-check

```bash
dotnet AniiSignal/build/AniiSignal.dll --selfcheck
```

Covers the URL-rewrite logic, which is the only part testable without the
game. Exits non-zero on failure, and `build-mod.sh` refuses to stage a build
that fails it. In-game, set `ANIISIGNAL_SELFCHECK=1` to run it at load.

## Configuration

`AniiSignal.json` sits next to the staged DLL. **Serviimo owns this file** -
once the server runs it rewrites the config with its real bind address and
gate public key, so there is one source of truth. The copy written by
`build-mod.sh` is a development default only.

| key | default | meaning |
|---|---|---|
| `CdnBaseUrl` | `http://127.0.0.1:8080` | where `server_list_publish.json` and the Lua/patch content are served |
| `OfficeBaseUrl` | `http://127.0.0.1:8081` | where the office API is served |
| `GatePublicKeyPem` | `""` | PKCS#1 PEM of the Serviimo gate keypair |
| `InjectGateKey` | `true` | substitute our key for FunPlus's in the client's `conf.json` |
| `RedirectEndpoints` | `true` | rewrite CDN/office hosts to the two URLs above |
| `StubAntiCheat` | `true` | stub the NetEase HTPSec token calls |
| `BlockOutboundTelemetry` | `false` | keep the game from reaching the real infrastructure |

## Status

| hook | state | notes |
|---|---|---|
| `EndpointRedirect` | working | patches `WebRequest.Create`, `WebClient.BaseAddress`, `HttpWebRequest.Address` - BCL types, so they bind without IL2CPP interop |
| `GateKeyInjector` | stub | validates the PEM and logs; the interop binding is pending. Until then use the CDN repack (`../PLAN.md` section 7) |
| `AntiCheatStub` | stub | names the target methods from the IL2CPP dump; binding pending |

Aniimo ships no `Managed/` assemblies, so game types do not exist at compile
time and Harmony cannot bind to them by `Type`. Game-side hooks therefore
resolve their targets by name at runtime, which is why they are stubs rather
than `[HarmonyPatch]` attributes on real types.
