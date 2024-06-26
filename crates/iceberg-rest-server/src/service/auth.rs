use super::{ProjectIdent, TableIdentUuid, WarehouseIdent};
use http::HeaderMap;
use iceberg_rest_service::v1::{NamespaceIdent, Result};

#[derive(Clone, Debug)]
pub enum UserID {
    Anonymous,
    User(String),
}

impl UserID {
    #[must_use]
    pub fn new(user_id: String) -> Self {
        Self::User(user_id)
    }

    #[must_use]
    pub fn new_anonymous() -> Self {
        Self::Anonymous
    }

    #[must_use]
    #[inline]
    pub fn is_anonymous(&self) -> bool {
        matches!(self, Self::Anonymous)
    }

    #[must_use]
    #[inline]
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User(_))
    }
}

#[derive(Debug, Clone)]
pub struct UserWarehouse {
    pub user_id: UserID,
    pub project_id: Option<ProjectIdent>,
    pub warehouse_id: Option<WarehouseIdent>,
}

#[derive(Debug, Clone)]
pub enum NamespacePermission {
    Read,
    Write,
}

#[async_trait::async_trait]
#[allow(clippy::module_name_repetitions)]
pub trait AuthZHandler
where
    Self: Sized + Send + Sync + Clone + 'static,
{
    type State: Clone + Send + Sync + 'static;

    async fn check_list_namespace(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        parent: Option<&NamespaceIdent>,
        state: Self::State,
    ) -> Result<()>;

    async fn check_create_namespace(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        parent: Option<&NamespaceIdent>,
        state: Self::State,
    ) -> Result<()>;

    async fn check_load_namespace_metadata(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        namespace: &NamespaceIdent,
        state: Self::State,
    ) -> Result<()>;

    /// Check if the user is allowed to check if a namespace exists,
    /// not check if the namespace exists.
    async fn check_namespace_exists(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        namespace: &NamespaceIdent,
        state: Self::State,
    ) -> Result<()>;

    async fn check_drop_namespace(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        namespace: &NamespaceIdent,
        state: Self::State,
    ) -> Result<()>;

    async fn check_update_namespace_properties(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        namespace: &NamespaceIdent,
        state: Self::State,
    ) -> Result<()>;

    async fn check_create_table(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        namespace: &NamespaceIdent,
        state: Self::State,
    ) -> Result<()>;

    async fn check_list_tables(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        namespace: &NamespaceIdent,
        state: Self::State,
    ) -> Result<()>;

    /// Check if the user is allowed to load a table.
    ///
    /// `table` is an optional argument because we might not be able
    /// to obtain a table-id from the table_name a user specifies.
    /// In most cases, unless the user has high permissions on a
    /// namespace, you would probably want to return 401.
    ///
    /// Arguments:
    /// - `warehouse_id`: The warehouse the table is in.
    /// - `namespace`: The namespace the table is in. (Direct parent)
    ///
    async fn check_load_table(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        namespace: Option<&NamespaceIdent>,
        table: Option<&TableIdentUuid>,
        state: Self::State,
    ) -> Result<()>;

    /// This should check if the user is allowed to rename the table.
    /// For rename to work, also "check_create_table" must pass
    /// for the destination namespace.
    async fn check_rename_table(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        source: Option<&TableIdentUuid>,
        state: Self::State,
    ) -> Result<()>;

    async fn check_table_exists(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        namespace: Option<&NamespaceIdent>,
        table: Option<&TableIdentUuid>,
        state: Self::State,
    ) -> Result<()>;

    async fn check_drop_table(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        table: Option<&TableIdentUuid>,
        state: Self::State,
    ) -> Result<()>;

    async fn check_commit_table(
        headers: &HeaderMap,
        warehouse_id: &WarehouseIdent,
        table: Option<&TableIdentUuid>,
        namespace: Option<&NamespaceIdent>,
        state: Self::State,
    ) -> Result<()>;
}

/// Interface to provide Auth-related functions to the config gateway.
/// This is separated from the AuthHandler as different functions
/// are required while fetching the config. The config server might be
/// external to the rest of the catalog.
// We use the same associated type as AuthHandler to avoid requiring
// an additional state to pass as part of the APIContext.
// A dummy AuthHandler implementation is enough to implement this trait.
// This still feels less clunky than using a generic state type.
#[async_trait::async_trait]
#[allow(clippy::module_name_repetitions)]
pub trait AuthConfigHandler<A: AuthZHandler>
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
        state: A::State,
        headers: &HeaderMap,
    ) -> Result<UserWarehouse>;

    /// Enrich / Exchange the token that is used for all further requests
    /// to the specified warehouse. Typically, this is used to enrich the
    /// token with the warehouse-id, so that the get_token function can
    /// extract it.
    /// If this AuthNHadler does not support enriching the token, or
    /// if no change to the original token is required, return Ok(None).
    async fn exchange_token_for_warehouse(
        state: A::State,
        previous_headers: &HeaderMap,
        project_id: &ProjectIdent,
        warehouse_id: &WarehouseIdent,
    ) -> Result<Option<String>>;

    // // Used for all endpoints
    // fn get_warehouse(state: T, headers: &HeaderMap) -> Result<WarehouseIdent>;

    /// Check if the user is allowed to list all warehouses in a project.
    async fn check_user_list_warehouse_in_project(
        state: A::State,
        user_id: &UserID,
        project_id: &ProjectIdent,
    ) -> Result<()>;

    /// Check if the user is allowed to get the config for a warehouse.
    async fn check_user_get_config_for_warehouse(
        state: A::State,
        user_id: &UserID,
        warehouse_id: &WarehouseIdent,
    ) -> Result<()>;
}
