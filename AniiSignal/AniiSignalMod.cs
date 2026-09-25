// AniiSignal - the client half of Serviimo.
//
// A MelonLoader mod whose only job is to make an unmodified Aniimo client
// connect to a personal-use server instead of FunPlus. See ../PLAN.md
// section 3 for the full vanilla/mod split.
//
// The rules this mod follows, and why:
//
//  1. It changes WHERE the client connects, never WHAT it sends. No payload
//     is rewritten anywhere in this project. A mod that edits messages makes
//     every later bug ambiguous between a Serviimo defect and a mod defect.
//
//  2. The vanilla path (no mod) stays the acceptance test for the server. The
//     mod is an accelerator for the three painful edges of that path - the
//     wine CA install, the patcher clobbering our XDF, and HTPSec phoning
//     home - not a dependency.
//
//  3. Every hook degrades to a log line instead of throwing during load, so a
//     game update that moves a target costs a feature, not the whole session.
//
// IL2CPP constraint, and the thing that shapes every file here: Aniimo ships
// no Managed/ assemblies, so game types do not exist at compile time and
// Harmony cannot bind to them by Type. Game-side hooks must resolve their
// targets by name from the runtime interop types, which is why they cannot be
// written as [HarmonyPatch] attributes against real types. BCL types are
// unaffected and bind normally.
//
// Status: folder foundation, comments only. Nothing implemented yet.

using MelonLoader;

// MelonGame values read from the globalgamemanagers header rather than
// guessed: companyName and productName are both "Aniimo".
[assembly: MelonGame("Aniimo", "Aniimo")]

namespace AniiSignal
{
    // Entry point. Loads the config, then applies each hook in isolation so
    // one failure cannot take down the rest.
    //
    // Apply order matters: the client reads the gate key while parsing config,
    // which happens while the first endpoint request is being built, so the
    // key must be in place before anything is allowed to dial out.
    //
    //   1. GateKeyInjector   our public key in place of FunPlus's
    //   2. EndpointRedirect  CDN and office base URLs
    //   3. AntiCheatStub     dummy token so the SDK stops phoning home
    public class AniiSignalMod : MelonMod
    {
    }
}
