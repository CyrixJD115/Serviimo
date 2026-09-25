//! The world simulation: server-authoritative ECS entity/property sync.
//! Tier 1 = empty world + one player. Tier 2 = persistence + static
//! entities. Tier 3 = gameplay (spawns/AI/combat). The property schemas
//! ship inside the client (CompactPropertySchema / ProtobufConst), and
//! our own gate gives plaintext visibility into every sync message.
