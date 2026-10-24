//! World-gate protocol types and offline property-sync inspection.
//!
//! Aniimo's authoritative player state arrives after `RoleService.CS_Login`
//! through schema declarations and incremental property updates. The inner
//! values use plain MessagePack after the protobuf envelope. This module
//! decodes that bounded outer value while preserving the original bytes; the
//! compact declaration schema and native lazy payload variants remain
//! intentionally uninterpreted until separately validated. No live write path
//! is implemented here.

pub use apiimo_network::{
    ConnectServerReq as ConnectServerRequest, CsLoginRequest as RoleLoginRequest,
};

const MAX_PROPERTY_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldAuthToken {
    pub token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldRpcRoute {
    pub service: &'static str,
    pub method: &'static str,
}

pub const CONNECT_SERVER_METHOD: u16 = 3;
pub const WORLD_RPC_ROUTES: &[WorldRpcRoute] = &[WorldRpcRoute {
    service: "RoleService",
    method: "CS_Login",
}];

/// Evidence-backed boundaries for the three state surfaces relevant to a
/// Homeland editor. Keeping these distinct prevents a local ClientInfo cache
/// rewrite from being presented as a server-side Homeland save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomelandStateSurface {
    ClientInfoCache,
    WorldPropertySync,
    EntityRpc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomelandStateCapability {
    pub surface: HomelandStateSurface,
    pub contains_authoritative_layout: bool,
    pub mutation_role: &'static str,
    pub evidence: &'static str,
}

pub const HOMELAND_STATE_CAPABILITIES: &[HomelandStateCapability] = &[
    HomelandStateCapability {
        surface: HomelandStateSurface::ClientInfoCache,
        contains_authoritative_layout: false,
        mutation_role: "none; local cache inspection and backup only",
        evidence: "a confirmed ornament move left the Android cache byte-identical while the move appeared in an independent macOS client session",
    },
    HomelandStateCapability {
        surface: HomelandStateSurface::WorldPropertySync,
        contains_authoritative_layout: true,
        mutation_role: "read and reconcile server-owned entity properties",
        evidence: "shipping clients declare and consume schema-driven world property updates after role login",
    },
    HomelandStateCapability {
        surface: HomelandStateSurface::EntityRpc,
        contains_authoritative_layout: true,
        mutation_role: "server-validated ornament and blueprint operations",
        evidence: "Android and macOS client chunks expose matching Homeland RPC and argument constants",
    },
];

pub fn homeland_state_capability(
    surface: HomelandStateSurface,
) -> &'static HomelandStateCapability {
    HOMELAND_STATE_CAPABILITIES
        .iter()
        .find(|capability| capability.surface == surface)
        .expect("every Homeland state surface must have a capability record")
}

/// Entity RPCs used by the shipping macOS/GameHub and Android Homeland editors.
///
/// These names and argument labels were recovered from both clients' Lua
/// chunks. Their protocol strings match across platforms; only the embedded
/// build paths differ. They describe the validation surface only; Serviimo
/// does not dispatch them to a production server or claim their inner value
/// encoding is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomelandRpcContract {
    pub method: &'static str,
    pub arguments: &'static [&'static str],
    pub purpose: &'static str,
    pub dispatch: &'static str,
}

pub const HOMELAND_RPC_CONTRACTS: &[HomelandRpcContract] = &[
    HomelandRpcContract {
        method: "RPC_CS_AddOrnament",
        arguments: &["ornamentInfo", "buildExtraData"],
        purpose: "place one Homeland ornament",
        dispatch: "pg.me:serverMsg",
    },
    HomelandRpcContract {
        method: "RPC_CS_AddOrnaments",
        arguments: &["ornamentsInfo", "buildExtraData"],
        purpose: "place a batch of Homeland ornaments",
        dispatch: "pg.me:serverMsg",
    },
    HomelandRpcContract {
        method: "RPC_CS_UpdateOrnament",
        arguments: &["ornamentId", "ornamentInfo", "buildExtraData"],
        purpose: "update one placed ornament",
        dispatch: "pg.me:serverMsg",
    },
    HomelandRpcContract {
        method: "RPC_CS_UpdateOrnaments",
        arguments: &["ornamentUpdateInfo", "buildExtraData"],
        purpose: "update a batch of placed ornaments",
        dispatch: "pg.me:serverMsg",
    },
    HomelandRpcContract {
        method: "RPC_CS_RemoveOrnament",
        arguments: &["ornamentId"],
        purpose: "remove one placed ornament",
        dispatch: "pg.me:serverMsg",
    },
    HomelandRpcContract {
        method: "RPC_CS_RemoveOrnaments",
        arguments: &["ornamentIds"],
        purpose: "remove a batch of placed ornaments",
        dispatch: "pg.me:serverMsg",
    },
    HomelandRpcContract {
        method: "RPC_CS_LiftOrnaments",
        arguments: &["liftInfo"],
        purpose: "adjust ornament elevation",
        dispatch: "pg.me:serverMsg",
    },
    HomelandRpcContract {
        method: "RPC_CS_BuildHomeBlueprint",
        arguments: &[
            "sourceType",
            "blueprintId",
            "targetPos3",
            "targetYawAngle",
            "areaId",
        ],
        purpose: "instantiate a saved Homeland blueprint",
        dispatch: "self:serverMsg with local callback",
    },
    HomelandRpcContract {
        method: "RPC_CS_UploadHomeBlueprint",
        arguments: &["ornamentIdList", "name", "desc", "coverImageKeys", "size"],
        purpose: "save or publish a Homeland blueprint",
        dispatch: "self:serverMsg with local callback",
    },
];

pub const ORNAMENT_POSITION_UNITS_PER_WORLD_UNIT: f64 = 100.0;
pub const ORNAMENT_ROTATION_UNITS_PER_DEGREE: f64 = 100.0;
pub const ORNAMENT_SCALE_UNITS_PER_ONE: f64 = 1000.0;

const ORNAMENT_FLAG_HAS_YAW: u16 = 0x0001;
const ORNAMENT_FLAG_HAS_PITCH_ROLL: u16 = 0x0002;
const ORNAMENT_FLAG_SCALE_UNIFORM: u16 = 0x0004;
const ORNAMENT_FLAG_SCALE_XYZ: u16 = 0x0008;
const ORNAMENT_FLAG_HAS_TRASH: u16 = 0x0010;
const ORNAMENT_FLAG_ELECTRIC_MODE: u16 = 0x0020;
const ORNAMENT_AREA_SHIFT: u16 = 6;
const ORNAMENT_AREA_MASK: u16 = 0x000f;
const ORNAMENT_FLAG_HAS_AREA_EXT: u16 = 0x0400;
const ORNAMENT_KNOWN_FLAGS: u16 = ORNAMENT_FLAG_HAS_YAW
    | ORNAMENT_FLAG_HAS_PITCH_ROLL
    | ORNAMENT_FLAG_SCALE_UNIFORM
    | ORNAMENT_FLAG_SCALE_XYZ
    | ORNAMENT_FLAG_HAS_TRASH
    | ORNAMENT_FLAG_ELECTRIC_MODE
    | (ORNAMENT_AREA_MASK << ORNAMENT_AREA_SHIFT)
    | ORNAMENT_FLAG_HAS_AREA_EXT;
const ORNAMENT_FIXED_BYTES: usize = 12;

/// Integer transform fields carried by the Homeland ornament wrappers.
///
/// Missing groups stay missing. This type intentionally supports decoding
/// only: the shipping client uses `math.round`, `math.floor`, and an angle
/// normalizer whose exact boundary behavior must be verified before Atlas can
/// safely produce outbound values.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WireOrnamentTransform {
    pub pos_x: Option<i64>,
    pub pos_y: Option<i64>,
    pub pos_z: Option<i64>,
    pub rot_x: Option<i64>,
    pub rot_y: Option<i64>,
    pub rot_z: Option<i64>,
    pub scale_x: Option<i64>,
    pub scale_y: Option<i64>,
    pub scale_z: Option<i64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OrnamentTransform {
    pub position: Option<[f64; 3]>,
    pub rotation_degrees: Option<[f64; 3]>,
    pub scale: Option<[f64; 3]>,
}

impl WireOrnamentTransform {
    pub fn decode(self) -> OrnamentTransform {
        OrnamentTransform {
            position: decode_triplet(
                self.pos_x,
                self.pos_y,
                self.pos_z,
                ORNAMENT_POSITION_UNITS_PER_WORLD_UNIT,
            ),
            rotation_degrees: decode_triplet(
                self.rot_x,
                self.rot_y,
                self.rot_z,
                ORNAMENT_ROTATION_UNITS_PER_DEGREE,
            ),
            scale: decode_triplet(
                self.scale_x,
                self.scale_y,
                self.scale_z,
                ORNAMENT_SCALE_UNITS_PER_ONE,
            ),
        }
    }
}

/// One `ornamentBinaryData` map value decoded from the official client's
/// compact `OrnamentCodec` format. The ornament instance ID is intentionally
/// absent: it is the enclosing property-map key, not part of this blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrnamentBinaryRecord {
    pub flags: u16,
    pub home_id: u32,
    pub transform: WireOrnamentTransform,
    pub trash_id: u16,
    pub area_id: u8,
    pub electric_mode: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrnamentCodecError {
    ExpectedBase64String,
    NonUtf8Base64,
    InvalidBase64(String),
    TooShort { bytes: usize },
    UnsupportedFlags { flags: u16 },
    ConflictingScaleFlags,
    WrongSize { expected: usize, actual: usize },
}

impl std::fmt::Display for OrnamentCodecError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExpectedBase64String => {
                write!(formatter, "ornament MessagePack value is not a string")
            }
            Self::NonUtf8Base64 => write!(formatter, "ornament Base64 string is not UTF-8"),
            Self::InvalidBase64(message) => write!(formatter, "invalid ornament Base64: {message}"),
            Self::TooShort { bytes } => {
                write!(formatter, "ornament record is too short: {bytes} bytes")
            }
            Self::UnsupportedFlags { flags } => {
                write!(
                    formatter,
                    "ornament record has unsupported flags 0x{flags:04x}"
                )
            }
            Self::ConflictingScaleFlags => {
                write!(
                    formatter,
                    "ornament record sets both uniform and XYZ scale flags"
                )
            }
            Self::WrongSize { expected, actual } => write!(
                formatter,
                "ornament record has wrong size: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for OrnamentCodecError {}

impl OrnamentBinaryRecord {
    pub fn decode_message_pack(value: &rmpv::Value) -> Result<Self, OrnamentCodecError> {
        let string = value.as_str().ok_or_else(|| match value {
            rmpv::Value::String(_) => OrnamentCodecError::NonUtf8Base64,
            _ => OrnamentCodecError::ExpectedBase64String,
        })?;
        Self::decode_base64(string)
    }

    /// Decode a MessagePack string's text content after the property envelope.
    /// The official client applies `phonestcore.base64Decode` before reading
    /// this little-endian binary record.
    pub fn decode_base64(encoded: &str) -> Result<Self, OrnamentCodecError> {
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| OrnamentCodecError::InvalidBase64(error.to_string()))?;
        Self::decode_bytes(&bytes)
    }

    pub fn decode_bytes(bytes: &[u8]) -> Result<Self, OrnamentCodecError> {
        if bytes.len() < ORNAMENT_FIXED_BYTES {
            return Err(OrnamentCodecError::TooShort { bytes: bytes.len() });
        }
        let flags = u16::from_le_bytes([bytes[0], bytes[1]]);
        let unsupported = flags & !ORNAMENT_KNOWN_FLAGS;
        if unsupported != 0 {
            return Err(OrnamentCodecError::UnsupportedFlags { flags: unsupported });
        }
        if flags & ORNAMENT_FLAG_SCALE_UNIFORM != 0 && flags & ORNAMENT_FLAG_SCALE_XYZ != 0 {
            return Err(OrnamentCodecError::ConflictingScaleFlags);
        }

        let mut expected = ORNAMENT_FIXED_BYTES;
        if flags & ORNAMENT_FLAG_HAS_YAW != 0 {
            expected += 2;
        }
        if flags & ORNAMENT_FLAG_HAS_PITCH_ROLL != 0 {
            expected += 4;
        }
        if flags & ORNAMENT_FLAG_SCALE_UNIFORM != 0 {
            expected += 2;
        } else if flags & ORNAMENT_FLAG_SCALE_XYZ != 0 {
            expected += 6;
        }
        if flags & ORNAMENT_FLAG_HAS_TRASH != 0 {
            expected += 2;
        }
        if flags & ORNAMENT_FLAG_HAS_AREA_EXT != 0 {
            expected += 1;
        }
        if bytes.len() != expected {
            return Err(OrnamentCodecError::WrongSize {
                expected,
                actual: bytes.len(),
            });
        }

        let home_id = read_u32(bytes, 2);
        let mut cursor = ORNAMENT_FIXED_BYTES;
        let mut rot_x = 0_i64;
        let mut rot_y = 0_i64;
        let mut rot_z = 0_i64;
        if flags & ORNAMENT_FLAG_HAS_YAW != 0 {
            rot_y = read_u16(bytes, cursor) as i64;
            cursor += 2;
        }
        if flags & ORNAMENT_FLAG_HAS_PITCH_ROLL != 0 {
            rot_x = read_u16(bytes, cursor) as i64;
            rot_z = read_u16(bytes, cursor + 2) as i64;
            cursor += 4;
        }

        let mut scales = [1_000_i64; 3];
        if flags & ORNAMENT_FLAG_SCALE_UNIFORM != 0 {
            let uniform = read_u16(bytes, cursor) as i64;
            scales = [uniform; 3];
            cursor += 2;
        } else if flags & ORNAMENT_FLAG_SCALE_XYZ != 0 {
            scales = [
                read_u16(bytes, cursor) as i64,
                read_u16(bytes, cursor + 2) as i64,
                read_u16(bytes, cursor + 4) as i64,
            ];
            cursor += 6;
        }

        let trash_id = if flags & ORNAMENT_FLAG_HAS_TRASH != 0 {
            let value = read_u16(bytes, cursor);
            cursor += 2;
            value
        } else {
            0
        };
        let inline_area = ((flags >> ORNAMENT_AREA_SHIFT) & ORNAMENT_AREA_MASK) as u8;
        let area_id = if flags & ORNAMENT_FLAG_HAS_AREA_EXT != 0 {
            bytes[cursor]
        } else {
            inline_area
        };

        Ok(Self {
            flags,
            home_id,
            transform: WireOrnamentTransform {
                pos_x: Some(read_i16(bytes, 6) as i64),
                pos_y: Some(read_i16(bytes, 8) as i64),
                pos_z: Some(read_i16(bytes, 10) as i64),
                rot_x: Some(rot_x),
                rot_y: Some(rot_y),
                rot_z: Some(rot_z),
                scale_x: Some(scales[0]),
                scale_y: Some(scales[1]),
                scale_z: Some(scales[2]),
            },
            trash_id,
            area_id,
            electric_mode: flags & ORNAMENT_FLAG_ELECTRIC_MODE != 0,
        })
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_i16(bytes: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn decode_triplet(x: Option<i64>, y: Option<i64>, z: Option<i64>, units: f64) -> Option<[f64; 3]> {
    Some([x? as f64 / units, y? as f64 / units, z? as f64 / units])
}

pub fn homeland_rpc_contract(method: &str) -> Option<&'static HomelandRpcContract> {
    HOMELAND_RPC_CONTRACTS
        .iter()
        .find(|contract| contract.method == method)
}

pub fn has_route(service: &str, method: &str) -> bool {
    WORLD_RPC_ROUTES
        .iter()
        .any(|route| route.service == service && route.method == method)
}

/// The outer protobuf envelope for `sendPropertyDeclare`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDeclaration {
    pub bundle_config: Option<Vec<u8>>,
    pub custom_type_config: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedPropertyDeclaration {
    pub bundle_config: Option<rmpv::Value>,
    pub custom_type_config: Option<rmpv::Value>,
}

impl PropertyDeclaration {
    pub fn decode(wire: &[u8]) -> Result<Self, PropertySyncError> {
        enforce_limit(wire)?;
        let message =
            apiimo_network::pb_decode(wire).map_err(PropertySyncError::InvalidProtobuf)?;
        let declaration = Self {
            bundle_config: message.bytes(1).map(ToOwned::to_owned),
            custom_type_config: message.bytes(2).map(ToOwned::to_owned),
        };
        if declaration.bundle_config.is_none() && declaration.custom_type_config.is_none() {
            return Err(PropertySyncError::EmptyDeclaration);
        }
        Ok(declaration)
    }

    /// Decode the two declaration tables exactly as the shipping
    /// `GateClientRpcHandler` does after protobuf decoding: one MessagePack
    /// value per present field. This is still inspection-only; interpreting
    /// the compact schema tuples is a separate, validated step.
    pub fn decode_configs(&self) -> Result<DecodedPropertyDeclaration, PropertySyncError> {
        Ok(DecodedPropertyDeclaration {
            bundle_config: self
                .bundle_config
                .as_deref()
                .map(decode_message_pack_value)
                .transpose()?,
            custom_type_config: self
                .custom_type_config
                .as_deref()
                .map(decode_message_pack_value)
                .transpose()?,
        })
    }
}

/// The outer protobuf envelope for `propertySync2Client`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertySyncUpdate {
    pub owner_id: Option<Vec<u8>>,
    pub property_id: i64,
    pub operation_type: i32,
    pub name: Vec<u8>,
    pub value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyOperation {
    Change,
    Add,
    Delete,
    Insert,
    Remove,
    Unsupported(i32),
}

/// The outer protobuf envelope for `propertyIdSync2Client`.
///
/// This names a property without carrying its value. It is decoded separately
/// so an inspection pipeline cannot accidentally treat an ID announcement as
/// a state mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyIdentityUpdate {
    pub owner_id: Option<Vec<u8>>,
    pub property_id: i64,
    pub name: Vec<u8>,
}

impl PropertyIdentityUpdate {
    pub fn decode(wire: &[u8]) -> Result<Self, PropertySyncError> {
        enforce_limit(wire)?;
        let message =
            apiimo_network::pb_decode(wire).map_err(PropertySyncError::InvalidProtobuf)?;
        let property_id = message
            .varint(2)
            .ok_or(PropertySyncError::MissingField("propertyid"))? as i64;
        let name = message
            .bytes(3)
            .ok_or(PropertySyncError::MissingField("name"))?
            .to_vec();
        if name.is_empty() {
            return Err(PropertySyncError::EmptyName);
        }
        Ok(Self {
            owner_id: message.bytes(1).map(ToOwned::to_owned),
            property_id,
            name,
        })
    }

    pub fn display_name(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

/// A preserved entity RPC envelope from either `entityMessage` or
/// `ecsMessage`. The parameter bytes intentionally remain opaque until the
/// game's schema-driven codec is reproduced and validated offline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityRpcEnvelope {
    pub entity_id: Option<Vec<u8>>,
    pub method_name: Option<Vec<u8>>,
    pub parameters: Vec<u8>,
    pub context: Option<Vec<u8>>,
}

impl EntityRpcEnvelope {
    pub fn decode(wire: &[u8]) -> Result<Self, PropertySyncError> {
        enforce_limit(wire)?;
        let message =
            apiimo_network::pb_decode(wire).map_err(PropertySyncError::InvalidProtobuf)?;
        let parameters = message
            .bytes(4)
            .ok_or(PropertySyncError::MissingField("parameters"))?
            .to_vec();
        Ok(Self {
            entity_id: message.bytes(1).map(ToOwned::to_owned),
            method_name: message.bytes(2).map(ToOwned::to_owned),
            parameters,
            context: message.bytes(7).map(ToOwned::to_owned),
        })
    }

    pub fn display_method_name(&self) -> Option<String> {
        self.method_name
            .as_deref()
            .map(|name| String::from_utf8_lossy(name).into_owned())
    }

    pub fn homeland_contract(&self) -> Option<&'static HomelandRpcContract> {
        let name = std::str::from_utf8(self.method_name.as_deref()?).ok()?;
        homeland_rpc_contract(name)
    }
}

impl PropertySyncUpdate {
    pub fn decode(wire: &[u8]) -> Result<Self, PropertySyncError> {
        enforce_limit(wire)?;
        let message =
            apiimo_network::pb_decode(wire).map_err(PropertySyncError::InvalidProtobuf)?;
        let property_id = message
            .varint(2)
            .ok_or(PropertySyncError::MissingField("propertyid"))? as i64;
        let operation_type = message
            .varint(3)
            .ok_or(PropertySyncError::MissingField("optype"))? as u32
            as i32;
        let name = message
            .bytes(4)
            .ok_or(PropertySyncError::MissingField("name"))?
            .to_vec();
        if name.is_empty() {
            return Err(PropertySyncError::EmptyName);
        }
        Ok(Self {
            owner_id: message.bytes(1).map(ToOwned::to_owned),
            property_id,
            operation_type,
            name,
            value: message.bytes(5).map(ToOwned::to_owned),
        })
    }

    /// A lossy label for diagnostics only. The original name bytes remain in
    /// `name` and must be used for any future round-trip implementation.
    pub fn display_name(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }

    /// ClientRealPropertyComponent's incremental dispatch table. The native
    /// `UNPACK` value 6 is deliberately retained as unsupported because this
    /// Lua path does not implement it.
    pub fn operation(&self) -> PropertyOperation {
        match self.operation_type {
            1 => PropertyOperation::Change,
            2 => PropertyOperation::Add,
            3 => PropertyOperation::Delete,
            4 => PropertyOperation::Insert,
            5 => PropertyOperation::Remove,
            value => PropertyOperation::Unsupported(value),
        }
    }

    /// Decode the incremental value using the same plain MessagePack layer as
    /// the official clients. A missing protobuf value remains `None`; an
    /// encoded MessagePack nil remains `Some(Value::Nil)`.
    pub fn decode_value(&self) -> Result<Option<rmpv::Value>, PropertySyncError> {
        self.value
            .as_deref()
            .map(decode_message_pack_value)
            .transpose()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertySyncError {
    TooLarge { bytes: usize },
    InvalidProtobuf(String),
    MissingField(&'static str),
    EmptyDeclaration,
    EmptyName,
    InvalidMessagePack(String),
    TrailingMessagePackData { bytes: usize },
}

impl std::fmt::Display for PropertySyncError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge { bytes } => {
                write!(formatter, "property payload is too large: {bytes} bytes")
            }
            Self::InvalidProtobuf(message) => {
                write!(formatter, "invalid property protobuf: {message}")
            }
            Self::MissingField(field) => write!(
                formatter,
                "property protobuf is missing required field {field}"
            ),
            Self::EmptyDeclaration => {
                write!(formatter, "property declaration contains no schema bytes")
            }
            Self::EmptyName => write!(formatter, "property update contains an empty name"),
            Self::InvalidMessagePack(message) => {
                write!(formatter, "invalid property MessagePack value: {message}")
            }
            Self::TrailingMessagePackData { bytes } => write!(
                formatter,
                "property MessagePack value has {bytes} trailing bytes"
            ),
        }
    }
}

impl std::error::Error for PropertySyncError {}

fn enforce_limit(wire: &[u8]) -> Result<(), PropertySyncError> {
    if wire.len() > MAX_PROPERTY_PAYLOAD_BYTES {
        return Err(PropertySyncError::TooLarge { bytes: wire.len() });
    }
    Ok(())
}

fn decode_message_pack_value(wire: &[u8]) -> Result<rmpv::Value, PropertySyncError> {
    let mut cursor = std::io::Cursor::new(wire);
    let value = rmpv::decode::read_value(&mut cursor)
        .map_err(|error| PropertySyncError::InvalidMessagePack(error.to_string()))?;
    let consumed = cursor.position() as usize;
    if consumed != wire.len() {
        return Err(PropertySyncError::TrailingMessagePackData {
            bytes: wire.len() - consumed,
        });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apiimo_network::PbMessage;

    #[test]
    fn known_world_route_is_available() {
        assert!(has_route("RoleService", "CS_Login"));
        assert!(!has_route("RoleService", "CS_DeleteEverything"));
    }

    #[test]
    fn homeland_rpc_registry_is_unique_and_keeps_blueprint_contract() {
        let mut methods = HOMELAND_RPC_CONTRACTS
            .iter()
            .map(|contract| contract.method)
            .collect::<Vec<_>>();
        methods.sort_unstable();
        methods.dedup();
        assert_eq!(methods.len(), HOMELAND_RPC_CONTRACTS.len());

        let blueprint = homeland_rpc_contract("RPC_CS_BuildHomeBlueprint").unwrap();
        assert_eq!(
            blueprint.arguments,
            [
                "sourceType",
                "blueprintId",
                "targetPos3",
                "targetYawAngle",
                "areaId"
            ]
        );

        let update = homeland_rpc_contract("RPC_CS_UpdateOrnament").unwrap();
        assert_eq!(
            update.arguments,
            ["ornamentId", "ornamentInfo", "buildExtraData"]
        );
        assert_eq!(update.dispatch, "pg.me:serverMsg");
    }

    #[test]
    fn decodes_wire_ornament_transform_for_native_scene_use() {
        let decoded = WireOrnamentTransform {
            pos_x: Some(125),
            pos_y: Some(-250),
            pos_z: Some(0),
            rot_x: Some(0),
            rot_y: Some(9_000),
            rot_z: Some(18_000),
            scale_x: Some(1_000),
            scale_y: Some(750),
            scale_z: Some(1_250),
        }
        .decode();

        assert_eq!(decoded.position, Some([1.25, -2.5, 0.0]));
        assert_eq!(decoded.rotation_degrees, Some([0.0, 90.0, 180.0]));
        assert_eq!(decoded.scale, Some([1.0, 0.75, 1.25]));
    }

    #[test]
    fn incomplete_wire_transform_groups_remain_absent() {
        let decoded = WireOrnamentTransform {
            pos_x: Some(100),
            pos_y: Some(200),
            ..WireOrnamentTransform::default()
        }
        .decode();
        assert_eq!(decoded.position, None);
        assert_eq!(decoded.rotation_degrees, None);
        assert_eq!(decoded.scale, None);
    }

    #[test]
    fn decodes_minimal_ornament_binary_record() {
        let encoded = rmpv::Value::from("AABA4gEAnP/IANT+");
        let record = OrnamentBinaryRecord::decode_message_pack(&encoded).unwrap();
        assert_eq!(record.home_id, 123_456);
        assert_eq!(record.area_id, 0);
        assert_eq!(record.trash_id, 0);
        assert!(!record.electric_mode);
        let transform = record.transform.decode();
        assert_eq!(transform.position, Some([-1.0, 2.0, -3.0]));
        assert_eq!(transform.rotation_degrees, Some([0.0, 0.0, 0.0]));
        assert_eq!(transform.scale, Some([1.0, 1.0, 1.0]));
    }

    #[test]
    fn decodes_all_optional_ornament_fields() {
        let record =
            OrnamentBinaryRecord::decode_base64("OwREMyIRAIAAAP9/KCNkADyMTASwBBQFKgAb").unwrap();
        assert_eq!(record.flags, 0x043b);
        assert_eq!(record.home_id, 0x1122_3344);
        assert_eq!(record.area_id, 27);
        assert_eq!(record.trash_id, 42);
        assert!(record.electric_mode);
        let transform = record.transform.decode();
        assert_eq!(transform.position, Some([-327.68, 0.0, 327.67]));
        assert_eq!(transform.rotation_degrees, Some([1.0, 90.0, 359.0]));
        assert_eq!(transform.scale, Some([1.1, 1.2, 1.3]));
    }

    #[test]
    fn decodes_uniform_scale_and_inline_area() {
        let record = OrnamentBinaryRecord::decode_base64("5AMHAAAAAQACAAMA3AU=").unwrap();
        assert_eq!(record.flags, 0x03e4);
        assert_eq!(record.home_id, 7);
        assert_eq!(record.area_id, 15);
        assert!(record.electric_mode);
        assert_eq!(record.transform.decode().scale, Some([1.5, 1.5, 1.5]));
    }

    #[test]
    fn rejects_ambiguous_or_wrong_sized_ornament_records() {
        let mut conflicting = vec![0_u8; ORNAMENT_FIXED_BYTES + 8];
        conflicting[0..2].copy_from_slice(
            &(ORNAMENT_FLAG_SCALE_UNIFORM | ORNAMENT_FLAG_SCALE_XYZ).to_le_bytes(),
        );
        assert_eq!(
            OrnamentBinaryRecord::decode_bytes(&conflicting),
            Err(OrnamentCodecError::ConflictingScaleFlags)
        );

        assert_eq!(
            OrnamentBinaryRecord::decode_bytes(&[0; 11]),
            Err(OrnamentCodecError::TooShort { bytes: 11 })
        );
        assert_eq!(
            OrnamentBinaryRecord::decode_bytes(&[0; 13]),
            Err(OrnamentCodecError::WrongSize {
                expected: 12,
                actual: 13
            })
        );

        let mut unsupported = vec![0_u8; ORNAMENT_FIXED_BYTES];
        unsupported[0..2].copy_from_slice(&0x0800_u16.to_le_bytes());
        assert_eq!(
            OrnamentBinaryRecord::decode_bytes(&unsupported),
            Err(OrnamentCodecError::UnsupportedFlags { flags: 0x0800 })
        );
        assert_eq!(
            OrnamentBinaryRecord::decode_message_pack(&rmpv::Value::from(42)),
            Err(OrnamentCodecError::ExpectedBase64String)
        );
    }

    #[test]
    fn client_info_cache_is_not_misclassified_as_homeland_layout() {
        let cache = homeland_state_capability(HomelandStateSurface::ClientInfoCache);
        assert!(!cache.contains_authoritative_layout);
        assert!(cache.mutation_role.contains("inspection"));

        let property_sync = homeland_state_capability(HomelandStateSurface::WorldPropertySync);
        assert!(property_sync.contains_authoritative_layout);

        let entity_rpc = homeland_state_capability(HomelandStateSurface::EntityRpc);
        assert!(entity_rpc.contains_authoritative_layout);
        assert!(entity_rpc.mutation_role.contains("server-validated"));
    }

    #[test]
    fn decodes_property_declaration_without_interpreting_schema_bytes() {
        let bundle = vec![0x81, 0xa4, b'h', b'o', b'm', b'e', 0x01];
        let custom = vec![0x91, 0x2a];
        let mut message = PbMessage::new();
        message.set_bytes(1, bundle.clone());
        message.set_bytes(2, custom.clone());

        let decoded = PropertyDeclaration::decode(&message.encode()).unwrap();
        assert_eq!(decoded.bundle_config, Some(bundle.clone()));
        assert_eq!(decoded.custom_type_config, Some(custom.clone()));

        let configs = decoded.decode_configs().unwrap();
        assert_eq!(
            configs.bundle_config,
            Some(rmpv::Value::Map(vec![(
                rmpv::Value::from("home"),
                rmpv::Value::from(1)
            )]))
        );
        assert_eq!(
            configs.custom_type_config,
            Some(rmpv::Value::Array(vec![rmpv::Value::from(42)]))
        );
    }

    #[test]
    fn decodes_property_update_and_preserves_lazy_value_bytes() {
        let owner = b"avatar:123456789012".to_vec();
        let lazy_value = vec![0x81, 0xa5, b'l', b'e', b'v', b'e', b'l', 0x0b];
        let mut message = PbMessage::new();
        message.set_bytes(1, owner.clone());
        message.set_varint(2, 4_294_967_300);
        message.set_varint(3, 2);
        message.set_bytes(4, b"homeCamp".to_vec());
        message.set_bytes(5, lazy_value.clone());

        let decoded = PropertySyncUpdate::decode(&message.encode()).unwrap();
        assert_eq!(decoded.owner_id, Some(owner));
        assert_eq!(decoded.property_id, 4_294_967_300);
        assert_eq!(decoded.operation_type, 2);
        assert_eq!(decoded.operation(), PropertyOperation::Add);
        assert_eq!(decoded.display_name(), "homeCamp");
        assert_eq!(decoded.value, Some(lazy_value));
        assert_eq!(
            decoded.decode_value().unwrap(),
            Some(rmpv::Value::Map(vec![(
                rmpv::Value::from("level"),
                rmpv::Value::from(11)
            )]))
        );
    }

    #[test]
    fn rejects_incomplete_updates() {
        let mut message = PbMessage::new();
        message.set_varint(2, 7);
        message.set_varint(3, 1);
        assert_eq!(
            PropertySyncUpdate::decode(&message.encode()),
            Err(PropertySyncError::MissingField("name"))
        );
    }

    #[test]
    fn distinguishes_missing_value_from_encoded_nil_and_rejects_trailing_data() {
        let missing = PropertySyncUpdate {
            owner_id: None,
            property_id: 1,
            operation_type: 0,
            name: b"ornaments".to_vec(),
            value: None,
        };
        assert_eq!(missing.decode_value().unwrap(), None);

        let encoded_nil = PropertySyncUpdate {
            value: Some(vec![0xc0]),
            ..missing.clone()
        };
        assert_eq!(encoded_nil.decode_value().unwrap(), Some(rmpv::Value::Nil));

        let trailing = PropertySyncUpdate {
            value: Some(vec![0xc0, 0xc0]),
            ..missing
        };
        assert_eq!(
            trailing.decode_value(),
            Err(PropertySyncError::TrailingMessagePackData { bytes: 1 })
        );

        let native_unpack = PropertySyncUpdate {
            operation_type: 6,
            ..encoded_nil
        };
        assert_eq!(native_unpack.operation(), PropertyOperation::Unsupported(6));
    }

    #[test]
    fn decodes_property_identity_without_inventing_a_value() {
        let owner = b"home:123456789012".to_vec();
        let mut message = PbMessage::new();
        message.set_bytes(1, owner.clone());
        message.set_varint(2, 91);
        message.set_bytes(3, b"ornaments".to_vec());

        let decoded = PropertyIdentityUpdate::decode(&message.encode()).unwrap();
        assert_eq!(decoded.owner_id, Some(owner));
        assert_eq!(decoded.property_id, 91);
        assert_eq!(decoded.display_name(), "ornaments");
    }

    #[test]
    fn decodes_entity_rpc_and_classifies_known_homeland_method() {
        let parameters = vec![0x92, 0x01, 0x80];
        let mut message = PbMessage::new();
        message.set_bytes(1, b"player:123456789012".to_vec());
        message.set_bytes(2, b"RPC_CS_UpdateOrnament".to_vec());
        message.set_bytes(4, parameters.clone());
        message.set_bytes(7, b"callback:17".to_vec());

        let decoded = EntityRpcEnvelope::decode(&message.encode()).unwrap();
        assert_eq!(decoded.parameters, parameters);
        assert_eq!(
            decoded.display_method_name().as_deref(),
            Some("RPC_CS_UpdateOrnament")
        );
        assert_eq!(
            decoded.homeland_contract().map(|contract| contract.purpose),
            Some("update one placed ornament")
        );
    }

    #[test]
    fn rejects_entity_rpc_without_required_parameter_bytes() {
        let mut message = PbMessage::new();
        message.set_bytes(2, b"RPC_CS_UpdateOrnament".to_vec());
        assert_eq!(
            EntityRpcEnvelope::decode(&message.encode()),
            Err(PropertySyncError::MissingField("parameters"))
        );
    }
}
