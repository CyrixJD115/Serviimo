## October 24 follow-up: owning-map evidence and offline edit boundary

The companion's local read-only research now covers **3,964 unique keyed OrnamentCodec records** in one captured owning map: 743 Production, 3,119 Residential, and 102 Seasonal records. This supersedes the earlier 47-record coverage figure without changing the separation between ClientInfo cache, world property synchronization, and entity RPC.

The original 47 reference records retained their keys across one independently checked client restart. That specific observation supports identity-aware local drafts; it does not prove universal lifetime stability, current server ownership binding, or stability of all 3,964 records across later restarts. Duplicate/stale memory copies must still be resolved through ownership rather than record shape alone.

An offline native-companion translation candidate now edits integer-centimeter position fields while preserving every other original record byte. The no-op check covers all 3,964 records, and reverse/overflow fixture checks pass. This is a local encoding candidate, not a Serviimo transport implementation or proof of general client floating-point quantization. No production RPC was dispatched and no game/server save was changed.

The bounded restart observation and complete-map counts support further protocol work, while byte-preserving offline translation remains distinct from authoritative writeback. Owner/property binding, fresh-capture conflict handling, server rejection behavior, and reversible persistence validation are unresolved. The companion's broader source and rendering progress is recorded separately in [AniiStudio PR #2](https://github.com/CyrixJD115/AniiStudio/pull/2).

**Attribution:** Sol / Sol 6.1 implemented the overnight companion capture/editor/model/test work. Astra's contribution is the prior protocol analysis and this technical review/writeup. These companion results do not imply that this PR's Rust implementation or its previously reported test totals changed overnight. No raw captures, account identifiers, runtime addresses, credentials, proprietary assets, or binaries accompany this update.
