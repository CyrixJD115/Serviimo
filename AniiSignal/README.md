# AniiSignal

The client half of Serviimo: a MelonLoader mod that makes an unmodified
Aniimo client connect to a personal-use server instead of FunPlus.

**Status: folder foundation, comments only. Nothing is implemented yet.**

## The one rule

The mod changes **where** the client connects, never **what** it sends. No
payload is rewritten anywhere in this project.

That is what keeps the vanilla path - no mod at all - a valid acceptance test
for the server. A mod that edits messages makes every later bug ambiguous:
you cannot tell a Serviimo defect from a mod defect. Full reasoning in
`../PLAN.md` section 3.

## Why a mod at all

The vanilla path works, but it has three painful edges: installing a CA into
the Proton wine prefix, the patcher overwriting our `LuaScripts.xdf`, and
HTPSec phoning home. The mod removes all three. It is an accelerator, not a
dependency - the goal stays an unmodified client completing its own login
screen against Serviimo.

## Layout

| file | role |
|---|---|
| `AniiSignalMod.cs` | MelonMod entry point; applies the hooks in order, each isolated |
| `AniiSignalConfig.cs` | the `AniiSignal.json` Serviimo writes at runtime |
| `EndpointRedirect.cs` | CDN and office host overrides - the highest-value hook |
| `GateKeyInjector.cs` | our public key in place of FunPlus's `conf.json` value |
| `AntiCheatStub.cs` | NetEase HTPSec token stubs |

## The constraint that shapes all of it

Aniimo is IL2CPP and ships no `Managed/` directory, so game types do not exist
at compile time and Harmony cannot bind to them by `Type`. Game-side hooks
must resolve their targets by name from the runtime interop types, which is
why they cannot be written as `[HarmonyPatch]` attributes against real types.

BCL types are unaffected. That is why `EndpointRedirect` can target
`WebRequest`, `WebClient` and `HttpWebRequest` and bind normally - it is the
one hook not blocked on interop.

## Suggested order

1. `EndpointRedirect` - unblocks the HTTPS problem wholesale, and the rewrite
   logic is testable without the game.
2. `GateKeyInjector` - the CDN repack cannot be written yet
   (`XdfContainer` is read-only), so this is currently the only route to
   supplying the client our key.
3. `AntiCheatStub` - only once the SDK is demonstrably in the way.

## Setup, when there is something to build

Needs MelonLoader 0.6.6's `net6` build in the game directory, since Aniimo is
IL2CPP. `MelonGame` is `("Aniimo", "Aniimo")`, read from the
`globalgamemanagers` header rather than guessed.

Build and staging are not set up yet; `tools/build-mod.sh` from the previous
pass was removed with the implementation.
