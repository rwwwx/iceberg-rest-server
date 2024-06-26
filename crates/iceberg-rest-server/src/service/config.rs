use super::{Catalog, ProjectIdent, WarehouseIdent};
use iceberg_rest_service::{CatalogConfig, Result};

#[async_trait::async_trait]
#[allow(clippy::module_name_repetitions)]
pub trait ConfigProvider<C: Catalog>
where
    Self: Clone + Send + Sync + 'static,
{
    async fn get_warehouse_by_name(
        warehouse_name: &str,
        project_id: &ProjectIdent,
        catalog_state: C::State,
    ) -> Result<WarehouseIdent>;

    async fn get_config_for_warehouse(
        warehouse_id: &WarehouseIdent,
        catalog_state: C::State,
    ) -> Result<CatalogConfig>;
}
