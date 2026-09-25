// Self-check for the pure URL-rewrite logic.
//
// Redirect() is the one piece of AniiSignal that can be verified without the
// game running, and a silent failure there sends the client to FunPlus while
// the log looks healthy. Run it with:
//
//   ANIISIGNAL_SELFCHECK=1 dotnet <path-to-AniiSignal.dll>
//
// or let the mod run it at load when that variable is set.

using System;

namespace AniiSignal
{
    public static class SelfCheck
    {
        public static int Run()
        {
            var config = new AniiSignalConfig
            {
                CdnBaseUrl = "http://127.0.0.1:8080",
                OfficeBaseUrl = "http://127.0.0.1:8081",
            };

            var failures = 0;
            failures += Check("cdn host + path", config,
                "https://worldx-global-cdn.kingsgroupgames.com/server_list/server_list_publish.json",
                "http://127.0.0.1:8080/server_list/server_list_publish.json");

            failures += Check("cdn bare host, no path", config,
                "https://worldx-global-cdn.kingsgroupgames.com",
                "http://127.0.0.1:8080");

            failures += Check("office host + path", config,
                "https://worldx-office-api.aniimo.com/profile-sessions/start",
                "http://127.0.0.1:8081/profile-sessions/start");

            failures += Check("backup cdn preferred", config,
                "https://worldx-global-cdn-backup.kingsgroupgames.com/server_list/server_list_publish.json",
                "http://127.0.0.1:8080/server_list/server_list_publish.json");

            failures += Check("query string preserved", config,
                "https://worldx-office-api.aniimo.com/api/v2/notice?ver=278da592",
                "http://127.0.0.1:8081/api/v2/notice?ver=278da592");

            failures += Check("unrelated host untouched", config,
                "https://events.appsflyer.com/v1.0/c2s/e", "https://events.appsflyer.com/v1.0/c2s/e");

            failures += Check("empty input safe", config, "", "");
            failures += Check("null input safe", config, null, null);

            // Trailing slash on our own base must not double up.
            var trailing = new AniiSignalConfig { CdnBaseUrl = "http://127.0.0.1:8080/" };
            failures += Check("our trailing slash trimmed", trailing,
                "https://worldx-global-cdn.kingsgroupgames.com/x",
                "http://127.0.0.1:8080/x");

            Console.WriteLine(failures == 0
                ? "AniiSignal selfcheck: PASS"
                : $"AniiSignal selfcheck: {failures} FAILURE(S)");
            return failures;
        }

        private static int Check(string name, AniiSignalConfig config, string input, string expected)
        {
            var actual = EndpointRedirect.Redirect(input, config);
            if (string.Equals(actual, expected, StringComparison.Ordinal))
            {
                Console.WriteLine($"  ok   {name}");
                return 0;
            }
            Console.WriteLine($"  FAIL {name}");
            Console.WriteLine($"       expected: {expected ?? "(null)"}");
            Console.WriteLine($"       actual  : {actual ?? "(null)"}");
            return 1;
        }
    }
}
