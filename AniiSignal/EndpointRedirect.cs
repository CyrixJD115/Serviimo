// Endpoint override: point the game at Serviimo instead of FunPlus.
//
// The game learns where to connect from data it downloads at runtime - the
// CDN server_list_publish.json and a conf.json carrying the gate public key.
// Intercepting those strings in flight leaves the client's own parsing and
// connection logic completely untouched, which is the cleanest form of the
// override.
//
// This is the highest-value hook in the mod, and the only one that can be
// written today without waiting on the IL2CPP interop types: WebRequest,
// WebClient and HttpWebRequest are all BCL types, so Harmony binds to them
// normally. Game-side classes (ProfileApiClient and friends) would need
// runtime name resolution instead.
//
// Hosts to claim, longest suffix first so a specific host wins over a general
// one. Each carries an is_ssl flag in the server-list format, so the CDN
// crate can be tested over plain HTTP before any TLS work:
//
//   worldx-global-cdn-backup.kingsgroupgames.com  -> CdnBaseUrl
//   worldx-global-cdn.kingsgroupgames.com         -> CdnBaseUrl
//   worldx-office-api-stage.aniimo.com             -> OfficeBaseUrl
//   worldx-office-api.yimo.com                     -> OfficeBaseUrl
//   worldx-office-api.aniimo.com                   -> OfficeBaseUrl
//
// Two edge cases worth remembering when this gets written, both of which
// produced real bugs in a first pass:
//
//   - The rewrite must consume the scheme. Replacing only the host leaves
//     "https://" in front of our own http:// base URL.
//   - A bare host with no trailing path must still match, so slice by index
//     rather than doing a string Replace on host + "/".
//
// Leaving unrelated hosts (analytics, crash reporting) untouched on purpose:
// see BlockOutboundTelemetry in the config, which is off by default so a
// misconfigured mod cannot silently break an otherwise working install.

namespace AniiSignal
{
    public static class EndpointRedirect
    {
    }
}
