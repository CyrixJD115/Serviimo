// AniiSignal - the client half of Serviimo.
//
// A MelonLoader mod whose only job is to make an unmodified Aniimo client
// connect to a personal-use server instead of FunPlus. See ../PLAN.md section
// 3 for the vanilla/mod split and the rules this mod follows:
//
//   1. The mod changes WHERE the client connects, never WHAT it sends.
//   2. Vanilla (no mod) stays the acceptance test for the server.
//   3. Every hook degrades to a log line instead of throwing during load, so a
//      missing target in a game update costs a feature, not the session.
//
// Hooks, in the order they matter:
//   - EndpointRedirect  override the CDN and office base URLs
//   - GateKeyInjector   hand the client our public key in place of FunPlus's
//   - AntiCheatStub     return a dummy token so the SDK stops phoning home

using System;
using System.Reflection;
using MelonLoader;

[assembly: MelonInfo(typeof(AniiSignal.AniiSignalMod), "AniiSignal", "0.1.0", "Serviimo")]
[assembly: MelonGame("Aniimo", "Aniimo")]

namespace AniiSignal
{
    /// <summary>
    /// Entry point when the assembly is run directly, which is how the
    /// self-check gets exercised without launching the game:
    ///
    ///   dotnet AniiSignal.dll --selfcheck
    ///
    /// MelonLoader loads the assembly as a library in game, so a Main here is
    /// never reached there.
    /// </summary>
    public static class EntryPoint
    {
        public static int Main(string[] args)
        {
            if (args.Length > 0 && args[0] == "--selfcheck")
                return SelfCheck.Run() == 0 ? 0 : 1;

            Console.WriteLine("AniiSignal - MelonLoader mod. Run with --selfcheck, or see ../PLAN.md.");
            return 0;
        }
    }

    public class AniiSignalMod : MelonMod
    {
        public override void OnInitializeMelon()
        {
            if (Environment.GetEnvironmentVariable("ANIISIGNAL_SELFCHECK") == "1")
            {
                var failures = SelfCheck.Run();
                if (failures > 0)
                    MelonLogger.Error($"AniiSignal: selfcheck reported {failures} failure(s)");
            }

            var config = AniiSignalConfig.Load();

            MelonLogger.Msg("--------------------------------------------------");
            MelonLogger.Msg($"AniiSignal 0.1.0 up");
            MelonLogger.Msg($"  config    : {AniiSignalConfig.ConfigPath}");
            MelonLogger.Msg($"  cdn       : {config.CdnBaseUrl}");
            MelonLogger.Msg($"  office    : {config.OfficeBaseUrl}");
            MelonLogger.Msg($"  gate key  : {(string.IsNullOrEmpty(config.GatePublicKeyPem) ? "(none)" : $"{config.GatePublicKeyPem.Length} bytes")}");
            MelonLogger.Msg("--------------------------------------------------");

            // Order matters: the client reads the gate key while parsing config,
            // which happens while the first endpoint request is being built. Get
            // the key in place before anything is allowed to dial out.
            if (config.InjectGateKey)
                Safe("gate key injection", () => GateKeyInjector.Apply(config));

            if (config.RedirectEndpoints)
                Safe("endpoint redirect", () => EndpointRedirect.Apply(config));

            if (config.StubAntiCheat)
                Safe("anti-cheat stub", () => AntiCheatStub.Apply());

            MelonLogger.Msg("AniiSignal: ready");
        }

        /// <summary>
        /// Runs one hook in isolation. A hook that throws here would abort
        /// MelonLoader's load phase and cost us every other hook, so nothing
        /// propagates.
        /// </summary>
        private static void Safe(string what, System.Action action)
        {
            try
            {
                action();
            }
            catch (System.Exception ex)
            {
                MelonLogger.Error($"AniiSignal: {what} failed and is disabled: {ex}");
            }
        }
    }
}
