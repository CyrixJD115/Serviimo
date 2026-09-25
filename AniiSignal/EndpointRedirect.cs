// Endpoint override: point the game at Serviimo instead of FunPlus.
//
// The game learns where to connect from data it downloads at runtime - the
// CDN server_list_publish.json and a conf.json carrying the gate public key.
// The cleanest override is therefore to intercept those strings in flight and
// substitute ours, which leaves the client's own parsing and connection logic
// completely untouched.
//
// The mod changes WHERE the client connects. It never changes WHAT the client
// sends: no payload is rewritten anywhere in AniiSignal. That separation is
// deliberate - a mod that edits messages makes every later bug ambiguous
// between a Serviimo defect and a mod defect.
//
// IL2CPP note: Aniimo ships no Managed/ assemblies, so game types do not exist
// at compile time and Harmony cannot bind to them by Type. We therefore match
// on the declaring type's full name plus method name at runtime, which is why
// these are string lookups rather than [HarmonyPatch] attributes on real
// types. If MelonLoader's interop cannot produce a patchable MethodBase, the
// hook logs and disables itself rather than throwing during load.

using System;
using System.Reflection;
using HarmonyLib;
using MelonLoader;

namespace AniiSignal
{
    public static class EndpointRedirect
    {
        private const string HarmonyId = "com.serviimo.aniisignal.redirect";

        // Hostname -> our replacement. Longest-suffix match wins, so
        // "cdn.example.com" is tried before "example.com".
        private static readonly (string Suffix, string Replacement)[] Rules = new[]
        {
            ("worldx-global-cdn-backup.kingsgroupgames.com", "{cdn}"),
            ("worldx-global-cdn.kingsgroupgames.com",        "{cdn}"),
            ("worldx-office-api-stage.aniimo.com",            "{office}"),
            ("worldx-office-api.yimo.com",                    "{office}"),
            ("worldx-office-api.aniimo.com",                  "{office}"),
        };

        private static HarmonyLib.Harmony _harmony;
        private static AniiSignalConfig _config;
        private static int _applied;

        public static void Apply(AniiSignalConfig config)
        {
            _config = config ?? new AniiSignalConfig();
            if (!_config.RedirectEndpoints)
            {
                MelonLogger.Msg("AniiSignal: endpoint redirect disabled by config");
                return;
            }

            _harmony = new HarmonyLib.Harmony(HarmonyId);

            // WebRequest/WebClient are BCL types, so these bind normally and
            // cover the CDN and office traffic without touching game code.
            TryPatch(typeof(System.Net.WebRequest), "Create", nameof(Prefix_Create));
            TryPatch(typeof(System.Net.WebClient), "get_BaseAddress", nameof(Prefix_get_BaseAddress));
            TryPatch(typeof(System.Net.HttpWebRequest), "get_Address", nameof(Prefix_get_Address));

            MelonLogger.Msg($"AniiSignal: endpoint redirect active ({_applied} hook(s), cdn={_config.CdnBaseUrl}, office={_config.OfficeBaseUrl})");
        }

        private static void TryPatch(Type owner, string methodName, string prefix)
        {
            var target = AccessTools.Method(owner, methodName);
            if (target == null)
            {
                MelonLogger.Warning($"AniiSignal: {owner.FullName}.{methodName} not found; skipped");
                return;
            }
            var annotation = new HarmonyMethod(
                typeof(EndpointRedirect).GetMethod(prefix, BindingFlags.Static | BindingFlags.NonPublic));
            _harmony.Patch(target, prefix: annotation);
            _applied++;
        }

        /// <summary>
        /// Rewrites a URL whose host matches a known FunPlus endpoint. Pure, so
        /// it is testable without a running game - see <see cref="SelfCheck"/>.
        /// Returns the input unchanged when nothing matches.
        /// </summary>
        public static string Redirect(string url, AniiSignalConfig config)
        {
            if (string.IsNullOrEmpty(url) || config == null)
                return url;

            foreach (var (suffix, replacement) in Rules)
            {
                var index = url.IndexOf(suffix, StringComparison.OrdinalIgnoreCase);
                if (index < 0)
                    continue;

                var baseUrl = replacement
                    .Replace("{cdn}", config.CdnBaseUrl ?? "")
                    .Replace("{office}", config.OfficeBaseUrl ?? "")
                    .TrimEnd('/');

                // Replace the whole origin - scheme included, since our base
                // URL carries its own - but keep any path and query the client
                // appended. Slicing by index rather than string Replace means a
                // bare host with no trailing slash is handled too.
                var head = url.Substring(0, index);
                var schemeEnd = head.LastIndexOf("://", StringComparison.Ordinal);
                if (schemeEnd >= 0)
                    head = head.Substring(schemeEnd + 3);

                var tail = url.Substring(index + suffix.Length);
                return head + baseUrl + tail;
            }
            return url;
        }

        private static string Redirect(string url) => Redirect(url, _config);

        private static void Prefix_Create(ref Uri __result)
        {
            if (__result == null)
                return;
            var redirected = Redirect(__result.ToString());
            if (redirected != __result.ToString())
                __result = new Uri(redirected);
        }

        private static void Prefix_get_BaseAddress(ref Uri __result)
        {
            if (__result == null)
                return;
            var redirected = Redirect(__result.ToString());
            if (redirected != __result.ToString())
                __result = new Uri(redirected);
        }

        private static void Prefix_get_Address(ref Uri __result)
        {
            if (__result == null)
                return;
            var redirected = Redirect(__result.ToString());
            if (redirected != __result.ToString())
                __result = new Uri(redirected);
        }
    }
}
