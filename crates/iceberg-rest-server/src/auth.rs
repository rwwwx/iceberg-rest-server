use crate::models::{ProjectIdent, WarehouseIdent};
use http::HeaderMap;
use iceberg_rest_service::v1::Result;

// Should be very cheap to clone
#[allow(clippy::module_name_repetitions)]
pub trait AuthState: Clone + Send + Sync + 'static {}

#[derive(Clone, Debug)]
pub struct UserID(String);

impl UserID {
    #[must_use]
    pub fn new(id: String) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct UserWarehouse {
    pub user_id: UserID,
    pub project_id: Option<ProjectIdent>,
    pub warehouse_id: Option<WarehouseIdent>,
}

/// Interface to provide Auth-related functions to the config gateway.
#[async_trait::async_trait]
#[allow(clippy::module_name_repetitions)]
pub trait AuthConfigHandler<T: AuthState>
where
    Self: Sized + Send + Sync + Clone + 'static,
{
    /// Extract information from the user credentials. Return an error if
    /// the user is not authenticated or if an expected extraction
    /// of information (e.g. project or warehouse) failed.
    /// If information is correctly not available, return None for the
    /// respective field. In this case project / warehouse must be passed
    /// as arguments to the config endpoint.
    /// If a warehouse_id is returned, a project_id must also be returned.
    ///
    /// If a project_id or warehouse_id is returned, this function must also check the
    /// `list_warehouse_in_project` permission for a project_id and the
    /// `get_config_for_warehouse` permission for a warehouse_id.
    async fn get_and_validate_user_warehouse(
        state: T,
        headers: &HeaderMap,
    ) -> Result<UserWarehouse>;

    /// Enrich / Exchange the token that is used for all further requests
    /// to the specified warehouse. Typically, this is used to enrich the
    /// token with the warehouse-id, so that the get_token function can
    /// extract it.
    /// If this AuthNHadler does not support enriching the token, or
    /// if no change to the original token is required, return Ok(None).
    async fn exchange_token_for_warehouse(
        state: T,
        previous_headers: &HeaderMap,
        project_id: &ProjectIdent,
        warehouse_id: &WarehouseIdent,
    ) -> Result<Option<String>>;

    // // Used for all endpoints
    // fn get_warehouse(state: T, headers: &HeaderMap) -> Result<WarehouseIdent>;

    /// Check if the user is allowed to list all warehouses in a project.
    async fn check_user_list_warehouse_in_project(
        state: T,
        user_id: &UserID,
        project_id: &ProjectIdent,
    ) -> Result<()>;

    /// Check if the user is allowed to get the config for a warehouse.
    async fn check_user_get_config_for_warehouse(
        state: T,
        user_id: &UserID,
        warehouse_id: &WarehouseIdent,
    ) -> Result<()>;
}