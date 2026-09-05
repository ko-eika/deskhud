use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex, RwLock,
        mpsc::{self, SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};

use deskhud_engine::HudConfigValue;
use sysinfo::{ProcessesToUpdate, System};

const HISTORY_LIMIT: usize = 60;

#[derive(Debug, Clone, PartialEq)]
pub struct SampleRequest {
    interval: Duration,
    process_name: String,
    application: bool,
}

impl SampleRequest {
    pub fn from_config(config: &HashMap<String, HudConfigValue>, application: bool) -> Self {
        let seconds = config
            .get("refresh_seconds")
            .and_then(HudConfigValue::as_f32)
            .unwrap_or(1.0)
            .clamp(0.5, 30.0);
        let bounded = |key: &str, max: usize| {
            config
                .get(key)
                .and_then(HudConfigValue::as_str)
                .unwrap_or("")
                .chars()
                .take(max)
                .collect()
        };
        Self {
            interval: Duration::from_secs_f32(seconds),
            process_name: bounded("process_name", 260),
            application,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProcessMetric {
    pub name: String,
    pub pid: Option<u32>,
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub enum ApplicationState {
    #[default]
    NotSelected,
    NotRunning {
        selector: String,
    },
    Running {
        name: String,
        pid: u32,
        cpu_percent: f32,
        memory_bytes: u64,
        matches: usize,
    },
}

#[derive(Debug, Clone, Default)]
pub struct SystemSnapshot {
    pub system_cpu: Option<f32>,
    pub memory_used: u64,
    pub memory_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub process_count: usize,
    pub uptime_seconds: u64,
    pub deskhud: ProcessMetric,
    pub application: ApplicationState,
    pub cpu_history: Vec<f32>,
    pub deskhud_history: Vec<f32>,
    pub application_history: Vec<f32>,
    pub process_names: Vec<String>,
}

pub struct Sampler {
    sender: SyncSender<()>,
    requests: Arc<Mutex<HashMap<String, (SampleRequest, Instant)>>>,
    snapshot: Arc<RwLock<SystemSnapshot>>,
}

impl Sampler {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        let requests = Arc::new(Mutex::new(HashMap::new()));
        let snapshot = Arc::new(RwLock::new(SystemSnapshot::default()));
        let worker_requests = Arc::clone(&requests);
        let output = Arc::clone(&snapshot);
        let _ = thread::Builder::new()
            .name("deskhud-system-sampler".into())
            .spawn(move || worker(receiver, worker_requests, output));
        Self {
            sender,
            requests,
            snapshot,
        }
    }

    pub fn request(&self, instance_id: &str, request: SampleRequest) {
        if let Ok(mut requests) = self.requests.lock() {
            requests.insert(instance_id.to_owned(), (request, Instant::now()));
        }
        match self.sender.try_send(()) {
            Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
        }
    }

    pub fn snapshot(&self) -> SystemSnapshot {
        self.snapshot
            .read()
            .map(|value| value.clone())
            .unwrap_or_default()
    }

    pub fn process_names(&self) -> Vec<String> {
        match self.sender.try_send(()) {
            Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
        }
        self.snapshot().process_names
    }
}

fn worker(
    receiver: mpsc::Receiver<()>,
    request_state: Arc<Mutex<HashMap<String, (SampleRequest, Instant)>>>,
    output: Arc<RwLock<SystemSnapshot>>,
) {
    let mut system = System::new();
    let current_pid = sysinfo::get_current_pid().ok();
    let (mut cpu_history, mut deskhud_history, mut application_history) =
        (VecDeque::new(), VecDeque::new(), VecDeque::new());
    while receiver.recv().is_ok() {
        let mut request = current_request(&request_state);
        let mut active_until = Instant::now() + request.interval.saturating_mul(2);
        loop {
            sample(
                &mut system,
                current_pid,
                &request,
                &output,
                &mut cpu_history,
                &mut deskhud_history,
                &mut application_history,
            );
            let deadline = Instant::now() + request.interval;
            loop {
                let now = Instant::now();
                if now >= deadline || now >= active_until {
                    break;
                }
                match receiver
                    .recv_timeout(deadline.min(active_until).saturating_duration_since(now))
                {
                    Ok(()) => {
                        request = current_request(&request_state);
                        active_until = Instant::now() + request.interval.saturating_mul(2);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            if Instant::now() >= active_until {
                break;
            }
        }
    }
}

fn current_request(requests: &Mutex<HashMap<String, (SampleRequest, Instant)>>) -> SampleRequest {
    let fallback = SampleRequest {
        interval: Duration::from_secs(1),
        process_name: String::new(),
        application: false,
    };
    let Ok(mut requests) = requests.lock() else {
        return fallback;
    };
    let now = Instant::now();
    requests.retain(|_, (request, seen)| {
        now.duration_since(*seen) <= request.interval.saturating_mul(2)
    });
    let Some(interval) = requests.values().map(|(request, _)| request.interval).min() else {
        return fallback;
    };
    let application = requests
        .values()
        .filter(|(request, _)| request.application)
        .max_by_key(|(_, seen)| *seen)
        .map(|(request, _)| request.clone());
    SampleRequest {
        interval,
        process_name: application
            .as_ref()
            .map(|request| request.process_name.clone())
            .unwrap_or_default(),
        application: application.is_some(),
    }
}

fn sample(
    system: &mut System,
    current_pid: Option<sysinfo::Pid>,
    request: &SampleRequest,
    output: &RwLock<SystemSnapshot>,
    cpu_history: &mut VecDeque<f32>,
    deskhud_history: &mut VecDeque<f32>,
    application_history: &mut VecDeque<f32>,
) {
    system.refresh_cpu_usage();
    system.refresh_memory();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let system_cpu = finite_percent(system.global_cpu_usage());
    push_history(cpu_history, system_cpu.unwrap_or(0.0));
    let deskhud = current_pid
        .and_then(|pid| system.process(pid))
        .map(|process| process_metric(process, current_pid.map(sysinfo::Pid::as_u32)))
        .unwrap_or_default();
    push_history(deskhud_history, deskhud.cpu_percent.unwrap_or(0.0));
    let application = select_application(system, request);
    let application_cpu = match application {
        ApplicationState::Running { cpu_percent, .. } => cpu_percent,
        _ => 0.0,
    };
    push_history(application_history, application_cpu);

    if let Ok(mut target) = output.write() {
        *target = SystemSnapshot {
            system_cpu,
            memory_used: system.used_memory(),
            memory_total: system.total_memory(),
            swap_used: system.used_swap(),
            swap_total: system.total_swap(),
            process_count: system.processes().len(),
            uptime_seconds: System::uptime(),
            deskhud,
            application,
            cpu_history: cpu_history.iter().copied().collect(),
            deskhud_history: deskhud_history.iter().copied().collect(),
            application_history: application_history.iter().copied().collect(),
            process_names: running_process_names(system),
        };
    }
}

fn select_application(system: &System, request: &SampleRequest) -> ApplicationState {
    let selector = request.process_name.trim();
    if selector.is_empty() {
        return ApplicationState::NotSelected;
    }
    let mut matches = system
        .processes()
        .iter()
        .filter(|(_, process)| {
            process
                .name()
                .to_string_lossy()
                .eq_ignore_ascii_case(selector)
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|(pid, _)| pid.as_u32());
    let count = matches.len();
    let Some((pid, process)) = matches.into_iter().next() else {
        return ApplicationState::NotRunning {
            selector: selector.to_owned(),
        };
    };
    ApplicationState::Running {
        name: process.name().to_string_lossy().into_owned(),
        pid: pid.as_u32(),
        cpu_percent: finite_percent(process.cpu_usage()).unwrap_or(0.0),
        memory_bytes: process.memory(),
        matches: count,
    }
}

fn running_process_names(system: &System) -> Vec<String> {
    let mut names = system
        .processes()
        .values()
        .map(|process| process.name().to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .collect::<Vec<_>>();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    names
}

fn process_metric(process: &sysinfo::Process, pid: Option<u32>) -> ProcessMetric {
    ProcessMetric {
        name: process.name().to_string_lossy().into_owned(),
        pid,
        cpu_percent: finite_percent(process.cpu_usage()),
        memory_bytes: Some(process.memory()),
    }
}

fn finite_percent(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 100.0))
}

fn push_history(history: &mut VecDeque<f32>, value: f32) {
    if history.len() == HISTORY_LIMIT {
        history.pop_front();
    }
    history.push_back(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_is_bounded() {
        let mut config = HashMap::new();
        config.insert("refresh_seconds".into(), HudConfigValue::Float(0.01));
        config.insert(
            "process_name".into(),
            HudConfigValue::String("x".repeat(500)),
        );
        let request = SampleRequest::from_config(&config, true);
        assert_eq!(request.interval, Duration::from_secs_f32(0.5));
        assert_eq!(request.process_name.len(), 260);
    }

    #[test]
    fn history_is_bounded() {
        let mut history = VecDeque::new();
        for value in 0..100 {
            push_history(&mut history, value as f32);
        }
        assert_eq!(history.len(), HISTORY_LIMIT);
        assert_eq!(history.front(), Some(&40.0));
    }

    #[test]
    fn active_instances_use_the_shortest_refresh_interval() {
        let requests = Mutex::new(HashMap::from([
            (
                "slow".to_owned(),
                (
                    SampleRequest {
                        interval: Duration::from_secs(5),
                        process_name: String::new(),
                        application: false,
                    },
                    Instant::now(),
                ),
            ),
            (
                "fast".to_owned(),
                (
                    SampleRequest {
                        interval: Duration::from_millis(500),
                        process_name: "app".to_owned(),
                        application: true,
                    },
                    Instant::now(),
                ),
            ),
        ]));
        let request = current_request(&requests);
        assert_eq!(request.interval, Duration::from_millis(500));
        assert_eq!(request.process_name, "app");
    }

    #[test]
    fn sysinfo_reports_real_system_and_current_process_data() {
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_processes(ProcessesToUpdate::All, true);
        assert!(system.total_memory() > 0);
        let pid = sysinfo::get_current_pid().expect("current pid");
        let process = system.process(pid).expect("current process");
        assert!(
            running_process_names(&system)
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&process.name().to_string_lossy()))
        );
    }
}
