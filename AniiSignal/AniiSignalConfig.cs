// AniiSignal configuration.
//
// The server is the source of truth: Serviimo writes this file next to the
// staged DLL at runtime, so the mod never hardcodes an address. There is one
// config, not two that drift.
//
// Read next to the assembly rather than from a fixed path, so the mod works
// regardless of where MelonLoader loads it from.
//
//   CdnBaseUrl              where server_list_publish.json and the Lua/patch
//                           content are served
//   OfficeBaseUrl           where the office API is served
//   GatePublicKeyPem        PKCS#1 PEM of the Serviimo gate keypair, injected
//                           in place of FunPlus's conf.json pubKeyData
//   InjectGateKey           substitute our key for the shipped one
//   RedirectEndpoints       rewrite CDN/office hosts to the two URLs above
//   StubAntiCheat           stub the NetEase HTPSec token calls
//   BlockOutboundTelemetry  keep the game from reaching the real
//                           infrastructure - off by default, so a misconfigured
//                           mod cannot silently break a working install
//
// All three booleans default to on except BlockOutboundTelemetry: a hook that
// fails should be visible in the log, and the safe fallback for a server that
// is not running yet is to do nothing rather than to point the client
// somewhere that does not answer.

namespace AniiSignal
{
    public sealed class AniiSignalConfig
    {
    }
}
