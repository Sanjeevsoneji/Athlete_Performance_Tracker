#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell, sync::atomic::{AtomicU64, Ordering}};

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

thread_local! {
    static ATHLETE_MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ATHLETE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

    static ATHLETE_STORAGE: RefCell<StableBTreeMap<u64, AthletePerformance, Memory>> =
        RefCell::new(StableBTreeMap::init(
            ATHLETE_MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

fn generate_athlete_id() -> u64 {
    ATHLETE_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn do_insert_athlete_performance(athlete: &AthletePerformance) -> Result<(), String> {
    ATHLETE_STORAGE.with(|service| {
        service
            .borrow_mut()
            .insert(athlete.id, athlete.clone())
            .map_err(|e| format!("Failed to insert athlete performance: {}", e))
    })
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct AthleteUpdatePayload {
    athlete_name: String,
    sport: String,
    performance_metrics: String,
    achievements: Vec<String>,
}

#[ic_cdk::query]
fn get_athlete_performance(id: u64) -> Result<AthletePerformance, String> {
    ATHLETE_STORAGE.with(|s| {
        s.borrow()
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("Athlete performance with id={} not found", id))
    })
}

#[ic_cdk::update]
fn add_athlete_performance(athlete: AthleteUpdatePayload) -> Result<AthletePerformance, String> {
    let id = generate_athlete_id();
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
    do_insert_athlete_performance(&athlete_performance)?;
    Ok(athlete_performance)
}

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

#[ic_cdk::update]
fn update_athlete_performance(id: u64, payload: AthleteUpdatePayload) -> Result<AthletePerformance, String> {
    ATHLETE_STORAGE.with(|service| {
        service
            .borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| format!("Athlete performance with id={} not found", id))
            .and_then(|athlete| {
                if !payload.athlete_name.is_empty() {
                    athlete.athlete_name = payload.athlete_name.clone();
                }
                if !payload.sport.is_empty() {
                    athlete.sport = payload.sport.clone();
                }
                if !payload.performance_metrics.is_empty() {
                    athlete.performance_metrics = payload.performance_metrics.clone();
                }
                if !payload.achievements.is_empty() {
                    athlete.achievements = payload.achievements.clone();
                }
                athlete.updated_at = Some(time());
                do_insert_athlete_performance(athlete)?;
                Ok(athlete.clone())
            })
    })
}

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

#[ic_cdk::update]
fn update_athlete_achievements(
    id: u64,
    new_achievements: Vec<String>,
) -> Result<AthletePerformance, String> {
    ATHLETE_STORAGE.with(|service| {
        service
            .borrow_mut()
            .get_mut(&id)
            .ok_or_else(|| format!("Athlete performance with id={} not found", id))
            .and_then(|athlete| {
                athlete.achievements = new_achievements;
                athlete.updated_at = Some(time());
                do_insert_athlete_performance(athlete)?;
                Ok(athlete.clone())
            })
    })
}

#[ic_cdk::query]
fn get_recently_updated_athletes() -> Vec<AthletePerformance> {
    const SEVEN_DAYS_IN_SECONDS: u64 = 7 * 24 * 60 * 60;

    ATHLETE_STORAGE.with(|service| {
        let borrow = service.borrow();
        borrow
            .iter()
            .filter_map(|(_, athlete)| {
                if let Some(updated_at) = athlete.updated_at {
                    let seven_days_ago

 = time() - SEVEN_DAYS_IN_SECONDS;
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

#[ic_cdk::query]
fn get_athlete_count() -> Result<u64, String> {
    let count = ATHLETE_STORAGE.with(|service| service.borrow().len() as u64);

    if count > 0 {
        Ok(count)
    } else {
        Err("No athletes found.".to_string())
    }
}

#[ic_cdk::update]
fn delete_athlete_performance(id: u64) -> Result<AthletePerformance, String> {
    ATHLETE_STORAGE.with(|service| {
        service
            .borrow_mut()
            .remove(&id)
            .ok_or_else(|| format!("Athlete performance with id={} not found", id))
    })
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

ic_cdk::export_candid!();
