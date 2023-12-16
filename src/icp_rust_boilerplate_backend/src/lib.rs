#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct AthletePerformance {
    id: u64,
    athlete_name: String,
    sport: String,
    performance_metrics: String,
    achievements: Vec<String>,
    created_at: u64,
    updated_at: Option<u64>,
}

// Implementing Storable and BoundedStorable traits for AthletePerformance
impl Storable for AthletePerformance {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for AthletePerformance {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// ... (existing thread-local variables and payload structure)

// New thread-local variables for our Athlete Performance Tracker app

thread_local! {
    static ATHLETE_MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ATHLETE_ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(ATHLETE_MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter for athlete performances")
    );

    static ATHLETE_STORAGE: RefCell<StableBTreeMap<u64, AthletePerformance, Memory>> =
        RefCell::new(StableBTreeMap::init(
            ATHLETE_MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

// Helper method to perform insert for AthletePerformance
fn do_insert_athlete_performance(athlete: &AthletePerformance) {
    ATHLETE_STORAGE.with(|service| service.borrow_mut().insert(athlete.id, athlete.clone()));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct AthleteUpdatePayload {
    athlete_name: String,
    sport: String,
    performance_metrics: String,
    achievements: Vec<String>,
}

// get_athlete_performance Function:
#[ic_cdk::query]
fn get_athlete_performance(id: u64) -> Result<AthletePerformance, Error> {
    match _get_athlete_performance(&id) {
        Some(athlete) => Ok(athlete),
        None => Err(Error::NotFound {
            msg: format!("an athlete performance with id={} not found", id),
        }),
    }
}

// _get_athlete_performance Function:
fn _get_athlete_performance(id: &u64) -> Option<AthletePerformance> {
    ATHLETE_STORAGE.with(|s| s.borrow().get(id))
}

// add_athlete_performance Function:
#[ic_cdk::update]
fn add_athlete_performance(athlete: AthleteUpdatePayload) -> Option<AthletePerformance> {
    let id = ATHLETE_ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter for athlete performances");
    let timestamp = time();
    let athlete_performance = AthletePerformance {
        id,
        athlete_name: athlete.athlete_name,
        sport: athlete.sport,
        performance_metrics: athlete.performance_metrics,
        achievements: athlete.achievements,
        created_at: timestamp,
        updated_at: None,
    };
    do_insert_athlete_performance(&athlete_performance);
    Some(athlete_performance)
}

// // 2.7.4 update_athlete_performance Function:
// #[ic_cdk::update]
// fn update_athlete_performance(id: u64, payload: AthleteUpdatePayload) -> Result<AthletePerformance, Error> {
//     match ATHLETE_STORAGE.with(|service| service.borrow().get(&id)) {
//         Some(mut athlete) => {
//             athlete.athlete_name = payload.athlete_name;
//             athlete.sport = payload.sport;
//             athlete.performance_metrics = payload.performance_metrics;
//             athlete.achievements = payload.achievements;
//             athlete.updated_at = Some(time());
//             do_insert_athlete_performance(&athlete);
//             Ok(athlete)
//         }
//         None => Err(Error::NotFound {
//             msg: format!(
//                 "couldn't update an athlete performance with id={}. performance not found",
//                 id
//             ),
//         }),
//     }
// }


// get_all_athlete_performances Function:
#[ic_cdk::query]
fn get_all_athlete_performances() -> Vec<AthletePerformance> {
    ATHLETE_STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .map(|(_, value)| value.clone())
            .collect()
    })
}

// update_athlete Function:
#[ic_cdk::update]
fn update_athlete_performance(
    id: u64,
    payload: AthleteUpdatePayload,
) -> Result<AthletePerformance, Error> {
    match ATHLETE_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut athlete) => {
            if !payload.athlete_name.is_empty() {
                athlete.athlete_name = payload.athlete_name;
            }
            if !payload.sport.is_empty() {
                athlete.sport = payload.sport;
            }
            if !payload.performance_metrics.is_empty() {
                athlete.performance_metrics = payload.performance_metrics;
            }
            if !payload.achievements.is_empty() {
                athlete.achievements = payload.achievements;
            }
            athlete.updated_at = Some(time());
            do_insert_athlete_performance(&athlete);
            Ok(athlete)
        }
        None => Err(Error::NotFound {
            msg: format!("couldn't update athlete with id={}. athlete not found", id),
        }),
    }
}

// search_athlete_by_name Function:
#[ic_cdk::query]
fn search_athlete_by_name(name: String) -> Vec<AthletePerformance> {
    ATHLETE_STORAGE.with(|service| {
        let borrow = service.borrow();
        borrow
            .iter()
            .filter_map(|(_, athlete)| {
                if athlete.athlete_name.contains(&name) {
                    Some(athlete.clone())
                } else {
                    None
                }
            })
            .collect()
    })
}

// search_athlete_by_sport Function:
#[ic_cdk::query]
fn search_athlete_by_sport(sport: String) -> Vec<AthletePerformance> {
    ATHLETE_STORAGE.with(|service| {
        let borrow = service.borrow();
        borrow
            .iter()
            .filter_map(|(_, athlete)| {
                if athlete.sport.contains(&sport) {
                    Some(athlete.clone())
                } else {
                    None
                }
            })
            .collect()
    })
}

// search_athlete_by_achievements Function:
#[ic_cdk::query]
fn search_athlete_by_achievements(achievement: String) -> Vec<AthletePerformance> {
    ATHLETE_STORAGE.with(|service| {
        let borrow = service.borrow();
        borrow
            .iter()
            .filter_map(|(_, athlete)| {
                if athlete.achievements.contains(&achievement) {
                    Some(athlete.clone())
                } else {
                    None
                }
            })
            .collect()
    })
}

// update_athlete_achievements Function:
#[ic_cdk::update]
fn update_athlete_achievements(
    id: u64,
    new_achievements: Vec<String>,
) -> Result<AthletePerformance, Error> {
    match ATHLETE_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut athlete) => {
            athlete.achievements = new_achievements;
            athlete.updated_at = Some(time());
            do_insert_athlete_performance(&athlete);
            Ok(athlete)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't update achievements for athlete with id={}. athlete not found",
                id
            ),
        }),
    }
}
// get_recently_updated_athletes Function:
#[ic_cdk::query]
fn get_recently_updated_athletes() -> Vec<AthletePerformance> {
    ATHLETE_STORAGE.with(|service| {
        let borrow = service.borrow();
        borrow
            .iter()
            .filter_map(|(_, athlete)| {
                if let Some(updated_at) = athlete.updated_at {
                    // Assume recently updated within the last 7 days
                    let seven_days_ago = time() - (7 * 24 * 60 * 60);
                    if updated_at > seven_days_ago {
                        Some(athlete.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    })
}

// ... (existing code)

// get_athlete_count Function:
#[ic_cdk::query]
fn get_athlete_count() -> Result<u64, Error> {
    let count = ATHLETE_STORAGE.with(|service| service.borrow().len() as u64);

    if count > 0 {
        Ok(count)
    } else {
        Err(Error::NotFound {
            msg: "No athletes found.".to_string(),
        })
    }
}

// delete_athlete_performance Function:
#[ic_cdk::update]
fn delete_athlete_performance(id: u64) -> Result<AthletePerformance, Error> {
    match ATHLETE_STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(athlete) => Ok(athlete),
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't delete an athlete performance with id={}. performance not found.",
                id
            ),
        }),
    }
}

// 2.7.7 enum Error:
#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

// To generate the Candid interface definitions for our canister
ic_cdk::export_candid!();
