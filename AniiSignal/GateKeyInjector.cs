// Gate public key injection.
//
// The client reads its gate public key from the "pubKeyData" field of
// conf.json, which Apiimo extracts from the shipping LuaScripts.xdf at
// xfs/luascripts/Network/conf.json. On a private server that value must be
// OUR key, or the client encrypts its session key to FunPlus and the gate
// never sees anything readable.
//
// Two routes to get our key in, and they are not exclusive:
//
//   - the CDN repack (PLAN.md section 7): rewrite the XDF and serve it. Works
//     with a fully vanilla client, but the patcher can overwrite the file on
//     the next content update.
//
//   - this hook: return our conf.json in memory as the config is read, so
//     there is nothing on disk to clobber. This is the better long-term
//     answer, and the reason the vanilla path is not a hard prerequisite.
//
// The repack is currently the blocker - nothing in the workspace can write an
// XDF yet (XdfContainer is read-only). Until it exists this hook is the only
// route, which is why it is worth finishing first.
//
// What it needs: locate the type that owns the conf.json read, Harmony-patch
// the getter returning that string, and return our JSON with pubKeyData
// replaced. Candidate holders seen in the disassembly and Lua layer are
// NetHandler (builds the gate config from Data.sys_config_data),
// XPatchRemoteConfigMgr, and ClientConfig.
//
// Validate the PEM at load rather than mid-handshake. A malformed key would
// otherwise surface much later as an unreadable session, which is a far more
// confusing symptom than a startup error.

namespace AniiSignal
{
    public static class GateKeyInjector
    {
    }
}
