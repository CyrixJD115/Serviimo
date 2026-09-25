// Anti-cheat stub.
//
// NetEase HTPSec, entered in the game as NTESRiskSecProtect, is a client-side
// sensor: getToken and getDataSign produce values the server validates, and
// ioctl is a native command channel. On Serviimo that validation is ours, so
// the tokens mean nothing - but the SDK still phones home, and it may also
// react to the injection MelonLoader performs. Stubbing it early avoids both.
//
// From the RE report: the client is a sensor and signer, enforcement is
// server-side via businessId tokens and signed payloads. Bypassing the client
// does not bypass a server check, which is fine here because there is no real
// server to bypass.
//
// Worth being clear about the risk this carries, since it is the one part of
// the mod that touches anti-cheat: instrumentation against a kernel-level
// anti-cheat is a real account-risk decision, and it belongs to the account
// owner, not to the tooling. On a private server the argument for doing it is
// that the SDK's outbound calls reach the real vendor infrastructure with the
// account's identity attached.
//
// Methods to target, per the IL2CPP dump:
//   getToken(timeout, businessId) -> AntiCheatResult
//   getDataSign(inputData)
//   ioctl(RequestCmdID, data)
//
// Also worth stubbing later, once they prove to be the thing actually
// blocking: crash reporting (CrashSight), analytics (AppsFlyer), and the
// FunPlus SDK bootstrap. None of these should be touched until they are
// actually in the way.

namespace AniiSignal
{
    public static class AntiCheatStub
    {
    }
}
