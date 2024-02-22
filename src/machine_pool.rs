use std::{collections::BTreeMap, fmt::Debug, sync::Arc};

use async_trait::async_trait;
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::components::machine::{Config, Machine, MachineCore, MachineError};

#[derive(Debug)]
pub enum VmManageError {
    NotFound,
    AlreadyExists,
    InternalError,
    DatabaseError(String),
    MachineError(String),
}

pub type VmManageResult<T> = Result<T, VmManageError>;

impl ToString for VmManageError {
    fn to_string(&self) -> String {
        match self {
            VmManageError::NotFound => "Volume not found".to_string(),
            VmManageError::AlreadyExists => "Volume already exists".to_string(),
            VmManageError::InternalError => "Internal error".to_string(),
            VmManageError::DatabaseError(s) => format!("Database error: {s}"),
            VmManageError::MachineError(s) => format!("Machine start error: {s}"),
        }
    }
}

impl From<MachineError> for VmManageError {
    fn from(e: MachineError) -> Self {
        VmManageError::MachineError(e.to_string())
    }
}

impl From<sqlx::Error> for VmManageError {
    fn from(e: sqlx::Error) -> Self {
        VmManageError::DatabaseError(e.to_string())
    }
}

/// Regulated basic functions that a Vm managing agent must have
#[async_trait]
pub trait VmManagePool {
    /// Configuration type that is used to boot the machine
    type ConfigType;
    type MachineIdentifier;
    async fn create_machine(
        &mut self,
        config: &Self::ConfigType,
    ) -> VmManageResult<Self::MachineIdentifier>;

    async fn start_machine(&self, vmid: &Self::MachineIdentifier) -> VmManageResult<()>;

    async fn pause_machine(&self, vmid: &Self::MachineIdentifier) -> VmManageResult<()>;

    async fn resume_machine(&self, vmid: &Self::MachineIdentifier) -> VmManageResult<()>;

    async fn stop_machine(&self, vmid: &Self::MachineIdentifier) -> VmManageResult<()>;

    async fn delete_machine(&mut self, vmid: &Self::MachineIdentifier) -> VmManageResult<()>;
}

pub struct FirecrackerVmManagePool {
    pool_id: Uuid,
    machines: BTreeMap<Uuid, Arc<Mutex<Machine>>>,
    conn: PgPool,
}

#[async_trait]
impl VmManagePool for FirecrackerVmManagePool {
    type ConfigType = Config;
    type MachineIdentifier = Uuid;
    async fn create_machine(&mut self, config: &Config) -> VmManageResult<Uuid> {
        let (machine, _exit_ch) = Machine::new(config.to_owned())?;
        let vmid = Uuid::new_v4();
        let core = machine.dump_into_core().map_err(|e| {
            VmManageError::MachineError(format!("Fail to dump into MachineCore: {}", e))
        })?;
        // add to memory
        self.machines.insert(vmid, Arc::new(Mutex::new(machine)));
        // add core to database
        self.add_core(&vmid, &core).await?;

        Ok(vmid)
    }

    async fn start_machine(&self, vmid: &Uuid) -> VmManageResult<()> {
        let machine = self.machines.get(&vmid).ok_or(VmManageError::NotFound)?;
        machine.lock().await.start().await?;
        Ok(())
    }

    async fn pause_machine(&self, vmid: &Uuid) -> VmManageResult<()> {
        let machine = self.machines.get(&vmid).ok_or(VmManageError::NotFound)?;
        machine.lock().await.pause().await?;
        Ok(())
    }

    async fn resume_machine(&self, vmid: &Uuid) -> VmManageResult<()> {
        let machine = self.machines.get(&vmid).ok_or(VmManageError::NotFound)?;
        machine.lock().await.resume().await?;
        Ok(())
    }

    async fn stop_machine(&self, vmid: &Uuid) -> VmManageResult<()> {
        let machine = self.machines.get(&vmid).ok_or(VmManageError::NotFound)?;
        machine.lock().await.shutdown().await?;
        machine.lock().await.stop_vmm().await?;
        Ok(())
    }

    async fn delete_machine(&mut self, vmid: &Uuid) -> VmManageResult<()> {
        let machine = self.machines.remove(&vmid).ok_or(VmManageError::NotFound)?;
        machine.lock().await.shutdown().await?;
        machine.lock().await.stop_vmm().await?;

        Ok(())
    }
}

pub(crate) const MAX_CACHE_CAPACITY_ENV: &'static str = "MAX_CACHE_CAPACITY";
pub(crate) const DEFAULT_MAX_CACHE_CAPACITY: u64 = 10000;

pub(crate) const MACHINE_CORE_TABLE_NAME: &'static str = "MACHINE_CORE_TABLE_NAME";
pub(crate) const DEFAULT_MACHINE_CORE_TABLE: &'static str = "machine_core";

pub(crate) const VM_CONFIG_TABLE_NAME: &'static str = "VM_CONFIG_TABLE_NAME";
pub(crate) const DEFAULT_VM_CONFIG_TABLE: &'static str = "vmconfig";

pub(crate) const SNAPSHOT_TABLE_NAME: &'static str = "SNAPSHOT_TABLE_NAME";
pub(crate) const DEFAULT_SNAPSHOT_TABLE: &'static str = "snapshots";

impl FirecrackerVmManagePool {
    fn machine_core_storage_table(&self) -> String {
        format!(
            "{}_{}",
            std::env::var(MACHINE_CORE_TABLE_NAME)
                .unwrap_or(DEFAULT_MACHINE_CORE_TABLE.to_string()),
            self.pool_id
        )
    }

    fn config_storage_table(&self) -> String {
        format!(
            "{}_{}",
            std::env::var(VM_CONFIG_TABLE_NAME).unwrap_or(DEFAULT_VM_CONFIG_TABLE.to_string()),
            self.pool_id
        )
    }

    fn snapshot_storage_table(&self) -> String {
        format!(
            "{}_{}",
            std::env::var(SNAPSHOT_TABLE_NAME).unwrap_or(DEFAULT_SNAPSHOT_TABLE.to_string()),
            self.pool_id
        )
    }

    async fn add_core(&self, vmid: &Uuid, core: &MachineCore) -> VmManageResult<()> {
        let machine_core_storage_table = self.machine_core_storage_table();
        sqlx::query("INSERT INTO $1 (vmid, machine_core) VALUES ($2, $3)")
            .bind(machine_core_storage_table)
            .bind(vmid)
            .bind(sqlx::types::Json(core.to_owned()))
            .execute(&self.conn)
            .await?;

        Ok(())
    }

    async fn delete_core(&self, vmid: &Uuid) -> VmManageResult<()> {
        let machine_core_storage_table = self.machine_core_storage_table();
        sqlx::query("DELETE FROM $1 WHERE vmid = $2")
            .bind(machine_core_storage_table)
            .bind(vmid)
            .execute(&self.conn)
            .await?;
        Ok(())
    }

    async fn restore_all(&mut self) -> VmManageResult<()> {
        let machine_core_storage_table = self.machine_core_storage_table();
        let elements = sqlx::query_as::<_, PgMachineCoreElement>("SELECT * FROM $1")
            .bind(machine_core_storage_table)
            .fetch_all(&self.conn).await?;
        
        for element in elements {
            let core = element.machine_core.0;
            let (machine, _exit_ch) = Machine::rebuild(core)?;
            self.machines.insert(element.vmid, Arc::new(Mutex::new(machine)));
        }

        Ok(())
    }
}

#[allow(unused)]
#[derive(sqlx::FromRow)]
struct PgMachineCoreElement {
    vmid: Uuid,
    machine_core: sqlx::types::Json<MachineCore>,
}

#[allow(unused)]
#[derive(sqlx::FromRow)]
struct PgVmConfigElement {
    vmid: Uuid,
    config: sqlx::types::Json<Config>,
}

/// Yet to be used
/// Snapshot of memory and vm state creation, deleting and storage
impl FirecrackerVmManagePool {
    pub fn create_snapshot(&self, vmid: &Uuid) -> VmManageResult<Uuid> {
        // Return snapshot id
        todo!()
    }

    pub fn create_machine_from_snapshot(&self, vmid: &Uuid, snapshot_id: &Uuid) -> VmManageResult<()> {
        todo!()
    }

    pub fn delete_snapshot(&self, vmid: &Uuid, snapshot_id: &Uuid) -> VmManageResult<()> {
        todo!()
    }
}