use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use sled::Transactional;
use sled::transaction::{ConflictableTransactionResult, TransactionalTree};
use std::time::SystemTime;

#[repr(u8)]
enum Trees {
    DoorCodes = 0x1,
    CallLogs = 0x2,
    PinAttempts = 0x3,
}

#[derive(Clone, Debug, Deserialize_repr)]
#[repr(u8)]
pub enum ConfigKeyspace {
    SUPER = 0x0,
    AllCallsDial = 0x1,
    DialPhoneNumber = 0x09,
    DoorkingCallerId = 0x10,
    LandlordCallerId = 0x11,
}

#[repr(u8)]
#[derive(FromPrimitive)]
enum SuperVersions {
    One = 1,
}

pub struct Config {
    db: sled::Db,
    calls: sled::Tree,
    doorcodes: sled::Tree,
    attempts: sled::Tree,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorCode {
    pub id: u64,
    pub enabled: bool,
    pub name: String,
    pub code: String,
}

fn doorcode_to_code_keyspace(code: String) -> Vec<u8> {
    let mut key = vec![0xffu8; 8];
    key.extend_from_slice(code.as_bytes());

    return key;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallRecord {
    pub date: SystemTime,
    pub from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinAttempt {
    pub date: SystemTime,
    pub code: String,
    pub name: String,
    pub success: bool,
}

impl Config {
    pub async fn new(db_path: String) -> Config {
        let db: sled::Db = sled::open(&db_path).unwrap();
        let doorcodes: sled::Tree = db.open_tree(&[Trees::DoorCodes as u8]).unwrap();
        let calls: sled::Tree = db.open_tree(&[Trees::CallLogs as u8]).unwrap();
        let attempts: sled::Tree = db.open_tree(&[Trees::PinAttempts as u8]).unwrap();

        let config = Config {
            db,
            calls,
            doorcodes,
            attempts,
        };

        {
            let super_value = config
                .db
                .get(&[ConfigKeyspace::SUPER as u8])
                .unwrap()
                .map(|x| SuperVersions::from_u8(x[0]).unwrap());

            'outer: {
                'none: {
                    match super_value {
                        None => break 'none,
                        Some(SuperVersions::One) => break 'outer,
                    }
                }

                println!("db: running initial population");
                config.set_bool(ConfigKeyspace::AllCallsDial, false);

                config.add_door_code(DoorCode {
                    id: 0,
                    enabled: true,
                    name: "haylin".to_string(),
                    code: "123456".to_string(),
                });
                config.add_door_code(DoorCode {
                    id: 0,
                    enabled: true,
                    name: "backup".to_string(),
                    code: "987654".to_string(),
                });

                config.set_string(ConfigKeyspace::DialPhoneNumber, "+11234567890".to_string());
                config.set_string(ConfigKeyspace::DoorkingCallerId, "+11234567890".to_string());
                config.set_string(ConfigKeyspace::LandlordCallerId, "+11234567890".to_string());

                let _ = config
                    .db
                    .insert(&[ConfigKeyspace::SUPER as u8], &[SuperVersions::One as u8]);
            }
        }

        return config;
    }

    pub fn set_string(&self, id: ConfigKeyspace, val: String) {
        println!("db: setting CONFIG::{:?} to {}", id, val);
        self.db
            .insert(&[id as u8], val.as_str())
            .expect("CONFIG failed to insert");
    }

    pub fn get_string(&self, id: ConfigKeyspace) -> String {
        str::from_utf8(&self.db.get(&[id as u8]).unwrap().unwrap())
            .expect("CONFIG failed to decode")
            .to_string()
    }

    pub fn get_door_codes(&self) -> Vec<DoorCode> {
        let iter = self.doorcodes.range([0u8; 8]..[0xffu8; 8]);

        iter.filter_map(|e| e.ok())
            .map(|(_k, v)| serde_json::from_slice(&v).unwrap())
            .collect()
    }

    pub fn set_bool(&self, id: ConfigKeyspace, val: bool) {
        println!("db: setting CONFIG::{:?} to {}", id, val);
        self.db
            .insert(&[id as u8], &[val as u8])
            .expect("CONFIG update failed to insert");
    }

    pub fn get_bool(&self, id: ConfigKeyspace) -> bool {
        self.db.get(&[id as u8]).unwrap().unwrap()[0] == 1u8
    }

    pub fn add_door_code(&self, doorcode: DoorCode) {
        let mut doorcode = doorcode.clone();
        doorcode.id = self.db.generate_id().unwrap();

        println!("db: adding DOORCODE::{} with {:?}", doorcode.id, doorcode);

        if self
            .doorcodes
            .get(doorcode_to_code_keyspace(doorcode.code.clone()))
            .unwrap()
            .is_some()
        {
            // Duplicate code
            println!(
                "db: fail adding DOORCODE::{} with {:?}",
                doorcode.id, doorcode
            );
            return;
        }

        self.doorcodes
            .transaction(
                |txn: &TransactionalTree| -> ConflictableTransactionResult<_> {
                    let doorcode = doorcode.clone();

                    let val = serde_json::to_string(&doorcode).unwrap();
                    txn.insert(&doorcode.id.to_be_bytes(), val.as_str())?;

                    txn.insert(doorcode_to_code_keyspace(doorcode.code), val.as_str())?;

                    Ok(())
                },
            )
            .expect("DOORCODE txn failed");
    }

    pub fn update_door_code(&self, doorcode: DoorCode) {
        println!("db: setting DOORCODE::{} with {:?}", doorcode.id, doorcode);

        self.doorcodes
            .transaction(
                |txn: &TransactionalTree| -> ConflictableTransactionResult<_> {
                    let new_doorcode = doorcode.clone();

                    let raw = txn.get(doorcode.id.to_be_bytes()).unwrap().unwrap();
                    let old_doorcode: DoorCode = serde_json::from_slice(&raw).unwrap();

                    if old_doorcode.code != new_doorcode.code {
                        txn.remove(doorcode_to_code_keyspace(old_doorcode.code))?;
                    }

                    let val = serde_json::to_string(&new_doorcode).unwrap();
                    txn.insert(&new_doorcode.id.to_be_bytes(), val.as_str())?;

                    txn.insert(doorcode_to_code_keyspace(new_doorcode.code), val.as_str())?;

                    Ok(())
                },
            )
            .expect("DOORCODE txn failed");
    }

    pub fn check_door_code(&self, code: String) -> bool {
        let id = self.db.generate_id().unwrap();
        (&self.doorcodes, &self.attempts)
            .transaction(
                |(door_txn, attempts_txn)| -> ConflictableTransactionResult<bool> {
                    let door: Option<DoorCode> = door_txn
                        .get(doorcode_to_code_keyspace(code.clone()))?
                        .map(|raw| serde_json::from_slice(&raw).unwrap());

                    let doorlog = match door {
                        None => PinAttempt {
                            date: SystemTime::now(),
                            code: code.clone(),
                            name: "".to_string(),
                            success: false,
                        },
                        Some(door) => PinAttempt {
                            date: SystemTime::now(),
                            code: code.clone(),
                            name: door.name.clone(),
                            success: door.enabled,
                        },
                    };

                    let val = serde_json::to_string(&doorlog).unwrap();
                    attempts_txn.insert(&id.to_be_bytes(), val.as_str())?;

                    Ok(doorlog.success)
                },
            )
            .expect("Door code check")
    }

    pub fn get_pin_log(&self, count: usize) -> Vec<PinAttempt> {
        self.attempts
            .iter()
            .rev()
            .take(count)
            .filter_map(|e| e.ok())
            .map(|(_k, v)| serde_json::from_slice(&v).unwrap())
            .collect()
    }

    pub fn delete_door_code(&self, id: u64) {
        self.doorcodes
            .transaction(
                |txn: &TransactionalTree| -> ConflictableTransactionResult<_> {
                    let raw = txn.get(id.to_be_bytes()).unwrap().unwrap();
                    let doorcode: DoorCode = serde_json::from_slice(&raw).unwrap();

                    println!("db: deleting DOORCODE::{} {:?}", doorcode.id, doorcode);

                    txn.remove(&id.to_be_bytes())?;
                    txn.remove(doorcode_to_code_keyspace(doorcode.code))?;

                    Ok(())
                },
            )
            .expect("DOORCODE delete txn failed");
    }

    pub fn log_call(&self, from: String) {
        let id = self.db.generate_id().unwrap();
        let call = CallRecord {
            date: SystemTime::now(),
            from,
        };

        let _ = self.calls.insert(
            &id.to_be_bytes(),
            serde_json::to_string(&call).unwrap().as_bytes(),
        );
    }

    pub fn get_call_log(&self, count: usize) -> Vec<CallRecord> {
        self.calls
            .iter()
            .rev()
            .take(count)
            .filter_map(|e| e.ok())
            .map(|(_k, v)| serde_json::from_slice(&v).unwrap())
            .collect()
    }
}
