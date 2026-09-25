//! The phonest gate protocol server (TCP, ports 10000/20000).
//!
//! Implements the server half of the wire protocol reversed in Apiimo:
//! [u32 LE total][u16 LE method][protobuf] framing, methods
//! 1/10 (requestEncryptToken / responseEncryptToken),
//! 2 (confirmEncryptKey - we hold the private key, so the session
//! RC4 key is trivially recovered), 3 (connectServer),
//! verifySdk + CS_Login, heartbeat (6), microservice (7/8).
//!
//! Reuses apiimo-network types (framing, PbMessage, rc4, ServerListPublish)
//! once the path dependency is enabled.
