pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_users_table;
mod m20240201_000001_create_groups_table;
mod m20240201_000002_create_projects_table;
mod m20240201_000003_create_js_errors_table;
mod m20240201_000004_create_network_errors_table;
mod m20240201_000005_create_performance_data_table;
mod m20240201_000006_create_track_tables;
mod m20240301_000001_create_event_stats_table;
mod m20240301_000002_create_event_definitions_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_users_table::Migration),
            Box::new(m20240201_000001_create_groups_table::Migration),
            Box::new(m20240201_000002_create_projects_table::Migration),
            Box::new(m20240201_000003_create_js_errors_table::Migration),
            Box::new(m20240201_000004_create_network_errors_table::Migration),
            Box::new(m20240201_000005_create_performance_data_table::Migration),
            Box::new(m20240201_000006_create_track_tables::Migration),
            Box::new(m20240301_000001_create_event_stats_table::Migration),
            Box::new(m20240301_000002_create_event_definitions_table::Migration),
        ]
    }
}
