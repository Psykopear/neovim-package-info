use crate::neovim::DependencyInfo;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Cache {
    map: HashMap<String, Vec<(String, String)>>,
    last_updated: Instant,
    duration: Duration,
}

impl Cache {
    pub fn new(duration: u64) -> Self {
        Cache {
            map: HashMap::new(),
            last_updated: Instant::now(),
            duration: Duration::from_secs(duration),
        }
    }

    pub fn update(&mut self, dependencies: &Vec<DependencyInfo>) {
        if self.last_updated.elapsed() > self.duration {
            self.map = HashMap::new();
        } else {
            for dep in dependencies {
                self.insert(&dep);
            }
        }
        self.last_updated = Instant::now();
    }

    pub fn insert(&mut self, dep: &DependencyInfo) {
        self.map.insert(dep.name.clone(), dep.latest.clone());
    }

    pub fn get(
        &self,
        dep: &DependencyInfo,
        check_dependency: &Fn(&DependencyInfo) -> Vec<(String, String)>,
    ) -> Vec<(String, String)> {
        match self.map.get(&dep.name) {
            Some(latest) => latest.clone(),
            None => check_dependency(&dep),
        }
    }
}
