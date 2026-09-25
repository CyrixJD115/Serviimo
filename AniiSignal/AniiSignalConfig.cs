// AniiSignal configuration.
//
// The server is the source of truth: Serviimo writes this file next to the
// staged DLL at startup, so the mod never hardcodes an address. If the file
// is missing we fall back to the defaults below, which assume the usual
// local development layout.

using System;
using System.IO;
using System.Text.Json;
using MelonLoader;

namespace AniiSignal
{
    public sealed class AniiSignalConfig
    {
        public string CdnBaseUrl { get; set; } = "http://127.0.0.1:8080";
        public string OfficeBaseUrl { get; set; } = "http://127.0.0.1:8081";

        // PEM (PKCS#1) of the gate keypair Serviimo generated. Injected
        // in-process as the client's conf.json pubKeyData, so the XDF repack
        // stops being load-bearing.
        public string GatePublicKeyPem { get; set; } = "";

        public bool InjectGateKey { get; set; } = true;
        public bool RedirectEndpoints { get; set; } = true;
        public bool StubAntiCheat { get; set; } = true;

        // Keep the game from reaching the real infrastructure. Off by default
        // so a misconfigured mod cannot silently break an otherwise working
        // install.
        public bool BlockOutboundTelemetry { get; set; } = false;

        public static string ConfigPath =>
            Path.Combine(AppContext.BaseDirectory, "AniiSignal.json");

        public static AniiSignalConfig Load()
        {
            try
            {
                if (File.Exists(ConfigPath))
                {
                    return JsonSerializer.Deserialize<AniiSignalConfig>(
                        File.ReadAllText(ConfigPath)) ?? new AniiSignalConfig();
                }
            }
            catch (Exception ex)
            {
                MelonLogger.Warning($"AniiSignal: could not read config ({ex.Message}); using defaults");
            }
            return new AniiSignalConfig();
        }
    }
}
