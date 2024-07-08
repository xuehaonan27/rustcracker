//! Machine pool, which manages a pool of active machines.
//! When a machine is no longer active, then the machine will be dump to database.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::config::GlobalConfig;
use crate::database::Database;
use crate::machine_dev::{Machine, MachineExportedConfig};

/*
pub struct MachinePool {
    machines: Mutex<HashMap<Uuid, Machine>>,
    db: Arc<Database>,
}

impl MachinePool {
    pub fn new(db: Arc<Database>) -> Self {
        MachinePool {
            machines: Mutex::new(HashMap::new()),
            db,
        }
    }

    pub async fn add_machine(&self, machine: Machine) -> Uuid {
        let uuid = Uuid::new_v4();
        self.machines.lock().unwrap().insert(uuid, machine);
        uuid
    }

    pub async fn get_machine(&self, uuid: Uuid) -> Option<Machine> {
        let mut machines = self.machines.lock().unwrap();
        if let Some(machine) = machines.remove(&uuid) {
            Some(machine)
        } else if let Some(config) = self.db.retrieve_config(uuid).await {
            Machine::rebuild(config).await.ok()
        } else {
            None
        }
    }

    pub async fn remove_machine(&self, uuid: Uuid) {
        if let Some(machine) = self.machines.lock().unwrap().remove(&uuid) {
            let config = machine.export_config();
            self.db.store_config(uuid, config).await;
        }
    }
}
*/
struct MachinePool {
    active_machines: HashMap<Uuid, Machine>,
    inactive_machines: HashMap<Uuid, (MachineExportedConfig, Option<u32>)>, // (Config, PID)
}

impl MachinePool {
    pub fn new() -> Self {
        MachinePool {
            active_machines: HashMap::new(),
            inactive_machines: HashMap::new(),
        }
    }

    pub async fn add_machine(&mut self, machine: Machine) -> Uuid {
        let uuid = Uuid::new_v4();
        self.active_machines.insert(uuid, machine);
        uuid
    }

    pub async fn remove_machine(&mut self, uuid: &Uuid) {
        if let Some(machine) = self.active_machines.remove(uuid) {
            let config = machine.export_config();
            let pid = machine.get_machine_pid().await;
            self.inactive_machines.insert(*uuid, (config, pid));
        }
    }

    pub async fn get_machine(&mut self, uuid: &Uuid) -> Option<&mut Machine> {
        if let Some(machine) = self.active_machines.get_mut(uuid) {
            return Some(machine);
        }
        if let Some((config, Some(pid))) = self.inactive_machines.remove(uuid) {
            // Check if the process still exists and belongs to the machine
            if let Ok(process) = std::process::Command::new("ps")
                .arg("-p")
                .arg(pid.to_string())
                .output()
            {
                // if process.stdout.contains(b"your_process_name") {
                //     // Recreate the machine with the existing process
                //     let child = Command::new("your_command")
                //         .spawn()
                //         .expect("Failed to spawn process");
                //     // let machine = Machine {
                //     //     agent: Agent::new(),
                //     //     config,
                //     //     local: LocalAsync::new(),
                //     //     frck: FirecrackerAsync::new(),
                //     //     jailer: None,
                //     //     child: Mutex::new(child),
                //     // };
                //     self.active_machines.insert(*uuid, machine);
                //     return self.active_machines.get_mut(uuid);
                // }
            }
        }
        None
    }
}

async fn main() {
    let mut machine_pool = MachinePool::new();
    // let machine = Machine {
    //     // Initialize your machine here
    //
    // };
    let machine: Machine;
    // let uuid = machine_pool.add_machine(machine).await;
    // machine_pool.remove_machine(&uuid).await;
    // if let Some(machine) = machine_pool.get_machine(&uuid).await {
    // Use the machine
    // }
}
