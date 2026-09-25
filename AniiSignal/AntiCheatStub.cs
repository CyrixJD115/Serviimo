// Anti-cheat stub.
//
// NetEase HTPSec (NTESRiskSecProtect) is a client-side sensor: getToken and
// getDataSign produce values the server validates. On Serviimo that validation
// is ours, so the tokens mean nothing - but the SDK still phones home, and it
// may also react to the injection MelonLoader performs. Stubbing it early
// avoids both.
//
// Same IL2CPP caveat as the other hooks: the game types are resolved at
// runtime, so the binding is a stub until the interop types can be inspected.

using MelonLoader;

namespace AniiSignal
{
    public static class AntiCheatStub
    {
        // Matches NTESRiskSecProtect in the IL2CPP dump. Verified present in
        // the 28,348-type dump; the method names come from the RE report.
        private const string ProtectType = "NTESRiskSecProtect";
        private const string TokenMethod = "getToken";
        private const string SignMethod = "getDataSign";

        public static void Apply()
        {
            // TODO(RE): Harmony-patch ProtectType's getToken/getDataSign to
            // return a successful AntiCheatResult without calling the SDK.
            // Until then this logs so the mod's intent is visible in-game and
            // nobody wonders why the stub is inert.
            MelonLogger.Msg(
                $"AniiSignal: anti-cheat stub armed for {ProtectType}.{TokenMethod} / " +
                $"{ProtectType}.{SignMethod} (binding pending)");
        }
    }
}
