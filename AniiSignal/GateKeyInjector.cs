// Gate public key injection.
//
// The client reads its gate public key from the "pubKeyData" field of
// conf.json, which Apiimo extracts from the shipping LuaScripts.xdf at
// xfs/luascripts/Network/conf.json. On a private server that value must be
// OUR key, or the client encrypts its session key to FunPlus and the gate
// never sees anything readable.
//
// Two ways to get it there:
//   - the CDN repack (Plan section 7): rewrite the XDF, works with a fully
//     vanilla client, but the patcher can overwrite it
//   - this hook: return our conf.json in memory as the config is read, so
//     there is nothing on disk to clobber
//
// The hook is a stub on purpose. Binding to the game's config reader needs
// the IL2CPP interop type, which only exists once MelonLoader has generated
// its proxy assemblies at runtime. Rather than guess a method name that would
// silently never bind, the resolution is left as the one clearly marked step
// below and the mod reports what it needs.

using System;
using MelonLoader;

namespace AniiSignal
{
    public static class GateKeyInjector
    {
        // Candidate type names seen in the disassembly and the Lua layer. The
        // first one that resolves at runtime wins; this is the only place that
        // needs a real name once we can read the interop types.
        private static readonly string[] ConfigHolderTypes =
        {
            "NetHandler",
            "XPatchRemoteConfigMgr",
            "ClientConfig",
        };

        public static void Apply(AniiSignalConfig config)
        {
            if (string.IsNullOrWhiteSpace(config.GatePublicKeyPem))
            {
                MelonLogger.Warning(
                    "AniiSignal: InjectGateKey is on but GatePublicKeyPem is empty; " +
                    "falling back to the CDN repack path (PLAN.md section 7)");
                return;
            }

            ValidatePem(config.GatePublicKeyPem);

            // TODO(RE): bind here once the IL2CPP proxy assemblies can be
            // inspected at runtime. The work is: locate the type above, Harmony-
            // patch the getter that returns the conf.json string, and return
            // our JSON with pubKeyData replaced. Nothing about the handshake
            // changes - only where the key comes from.
            MelonLogger.Msg(
                "AniiSignal: gate key ready to inject but the interop binding is " +
                "not written yet; use the CDN repack for now");

            foreach (var name in ConfigHolderTypes)
                MelonLogger.Msg($"AniiSignal:   candidate config holder: {name}");
        }

        /// <summary>
        /// Fails loudly at load rather than mid-handshake. A malformed PEM here
        /// would otherwise surface as an unreadable session much later.
        /// </summary>
        private static void ValidatePem(string pem)
        {
            var body = pem
                .Replace("-----BEGIN PUBLIC KEY-----", "")
                .Replace("-----END PUBLIC KEY-----", "")
                .Replace("-----BEGIN RSA PUBLIC KEY-----", "")
                .Replace("-----END RSA PUBLIC KEY-----", "")
                .Replace("-----BEGIN CERTIFICATE-----", "")
                .Replace("-----END CERTIFICATE-----", "")
                .Replace("\n", "")
                .Replace("\r", "")
                .Trim();

            if (body.Length == 0)
                throw new ArgumentException("GatePublicKeyPem contains no base64 body");
            if (body.Length % 4 != 0)
                throw new ArgumentException(
                    $"GatePublicKeyPem base64 length {body.Length} is not a multiple of 4");

            MelonLogger.Msg($"AniiSignal: gate public key looks sane ({body.Length} base64 chars)");
        }
    }
}
