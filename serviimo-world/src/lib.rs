//! World-gate protocol types only. Simulation and persistence are later tiers.

pub use apiimo_network::{
    ConnectServerReq as ConnectServerRequest, CsLoginRequest as RoleLoginRequest,
};

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

pub fn has_route(service: &str, method: &str) -> bool {
    WORLD_RPC_ROUTES
        .iter()
        .any(|route| route.service == service && route.method == method)
}
