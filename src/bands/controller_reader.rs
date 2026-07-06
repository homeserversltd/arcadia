const CONTROLLER_READER_AXIS_DEADZONE: i16 = 6000;
const CONTROLLER_READER_EVENT_HISTORY_LIMIT: usize = 256;
const CONTROLLER_READER_REOPEN_DELAY_MS: u64 = 250;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ControllerJsEvent {
    time: u32,
    value: i16,
    kind: u8,
    number: u8,
}

impl ControllerJsEvent {
    fn event_type(self) -> u8 {
        self.kind & 0x7f
    }

    fn is_initial_state(self) -> bool {
        self.kind & 0x80 != 0
    }

    fn is_button(self) -> bool {
        self.event_type() == 0x01
    }

    fn is_axis(self) -> bool {
        self.event_type() == 0x02
    }
}

#[derive(Clone, Debug)]
struct ControllerButtonEdge {
    sequence: u64,
    number: u8,
}

#[derive(Clone, Copy, Debug)]
enum ControllerSnapshotConsumer {
    Regular,
    Trainer,
}

#[derive(Debug)]
struct ControllerReaderState {
    connected: bool,
    held_buttons: BTreeSet<u8>,
    axes: BTreeMap<u8, i16>,
    button_edges: std::collections::VecDeque<ControllerButtonEdge>,
    next_sequence: u64,
    last_regular_snapshot_sequence: u64,
    last_trainer_snapshot_sequence: u64,
}

impl ControllerReaderState {
    fn new() -> Self {
        Self {
            connected: false,
            held_buttons: BTreeSet::new(),
            axes: BTreeMap::new(),
            button_edges: std::collections::VecDeque::new(),
            next_sequence: 1,
            last_regular_snapshot_sequence: 0,
            last_trainer_snapshot_sequence: 0,
        }
    }

    fn mark_open(&mut self) {
        self.connected = true;
    }

    fn mark_closed(&mut self) {
        self.connected = false;
        self.held_buttons.clear();
        self.axes.clear();
        self.button_edges.clear();
        let last_sequence = self.next_sequence.saturating_sub(1);
        self.last_regular_snapshot_sequence = last_sequence;
        self.last_trainer_snapshot_sequence = last_sequence;
    }

    fn apply_event(&mut self, event: ControllerJsEvent) {
        if event.is_button() {
            if event.value != 0 {
                let was_new_press = self.held_buttons.insert(event.number);
                if was_new_press && !event.is_initial_state() {
                    self.push_button_edge(event.number);
                }
            } else {
                self.held_buttons.remove(&event.number);
            }
        } else if event.is_axis() {
            self.axes.insert(event.number, event.value);
        }
    }

    fn snapshot_bindings(&mut self) -> (bool, Vec<ControllerBindingStatus>, Vec<ControllerBindingStatus>) {
        self.snapshot_bindings_for(ControllerSnapshotConsumer::Regular)
    }

    fn snapshot_bindings_for(
        &mut self,
        consumer: ControllerSnapshotConsumer,
    ) -> (bool, Vec<ControllerBindingStatus>, Vec<ControllerBindingStatus>) {
        let last_snapshot_sequence = match consumer {
            ControllerSnapshotConsumer::Regular => self.last_regular_snapshot_sequence,
            ControllerSnapshotConsumer::Trainer => self.last_trainer_snapshot_sequence,
        };
        let edges_since_snapshot: Vec<u8> = self
            .button_edges
            .iter()
            .filter(|edge| edge.sequence > last_snapshot_sequence)
            .map(|edge| edge.number)
            .collect();
        let new_last_sequence = self.next_sequence.saturating_sub(1);
        match consumer {
            ControllerSnapshotConsumer::Regular => self.last_regular_snapshot_sequence = new_last_sequence,
            ControllerSnapshotConsumer::Trainer => self.last_trainer_snapshot_sequence = new_last_sequence,
        }
        self.prune_delivered_history();

        let mut pressed_numbers: BTreeSet<u8> = self.held_buttons.clone();
        pressed_numbers.extend(edges_since_snapshot);
        let pressed = pressed_numbers
            .into_iter()
            .map(|number| ControllerBindingStatus {
                control: format!("Button {}", number),
                binding: format!("button {}", number),
                pressed: true,
                axis_value: None,
            })
            .collect();
        let axes = self
            .axes
            .iter()
            .filter(|(_, value)| value.abs() > CONTROLLER_READER_AXIS_DEADZONE)
            .map(|(number, value)| ControllerBindingStatus {
                control: format!("Axis {}", number),
                binding: format!("axis {}", number),
                pressed: true,
                axis_value: Some(*value),
            })
            .collect();
        (self.connected, pressed, axes)
    }

    fn push_button_edge(&mut self, number: u8) {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.button_edges.push_back(ControllerButtonEdge { sequence, number });
        while self.button_edges.len() > CONTROLLER_READER_EVENT_HISTORY_LIMIT {
            self.button_edges.pop_front();
        }
    }

    fn prune_delivered_history(&mut self) {
        let delivered_floor = self
            .last_regular_snapshot_sequence
            .min(self.last_trainer_snapshot_sequence);
        while self.button_edges.len() > CONTROLLER_READER_EVENT_HISTORY_LIMIT / 2 {
            if self
                .button_edges
                .front()
                .is_some_and(|edge| edge.sequence <= delivered_floor)
            {
                self.button_edges.pop_front();
            } else {
                break;
            }
        }
    }
}

struct ControllerReaderHandle {
    state: Arc<Mutex<ControllerReaderState>>,
}

static CONTROLLER_READER_REGISTRY: OnceLock<Mutex<HashMap<String, Arc<ControllerReaderHandle>>>> = OnceLock::new();

fn controller_reader_registry() -> &'static Mutex<HashMap<String, Arc<ControllerReaderHandle>>> {
    CONTROLLER_READER_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn controller_reader_snapshot(device: Option<&ControllerDeviceStatus>) -> ControllerInputStatus {
    controller_reader_snapshot_for(device, ControllerSnapshotConsumer::Regular)
}

fn controller_reader_snapshot_fast(device: Option<&ControllerDeviceStatus>) -> ControllerInputStatus {
    controller_reader_snapshot_for(device, ControllerSnapshotConsumer::Trainer)
}

fn controller_reader_snapshot_for(
    device: Option<&ControllerDeviceStatus>,
    consumer: ControllerSnapshotConsumer,
) -> ControllerInputStatus {
    let Some(device) = device else {
        return ControllerInputStatus {
            state: "waiting".to_string(),
            device: "No controller detected".to_string(),
            sample_path: String::new(),
            pressed: Vec::new(),
            axes: Vec::new(),
        };
    };
    let sample_path = controller_joydev_path(device);
    let handle = controller_reader_for_path(&sample_path);
    let (connected, pressed, axes) = handle
        .state
        .lock()
        .map(|mut state| state.snapshot_bindings_for(consumer))
        .unwrap_or((false, Vec::new(), Vec::new()));
    ControllerInputStatus {
        state: if !connected && !Path::new(&sample_path).exists() {
            "waiting"
        } else if pressed.is_empty() && axes.is_empty() {
            "listening"
        } else {
            "active"
        }
        .to_string(),
        device: device.name.clone(),
        sample_path,
        pressed,
        axes,
    }
}

fn controller_joydev_path(device: &ControllerDeviceStatus) -> String {
    if device.path.contains("/dev/input/js") {
        device.path.clone()
    } else {
        first_js_path().unwrap_or_else(|| device.path.clone())
    }
}

fn controller_reader_for_path(path: &str) -> Arc<ControllerReaderHandle> {
    let registry = controller_reader_registry();
    let mut registry = registry.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(handle) = registry.get(path) {
        return Arc::clone(handle);
    }
    let handle = Arc::new(ControllerReaderHandle {
        state: Arc::new(Mutex::new(ControllerReaderState::new())),
    });
    spawn_controller_reader(path.to_string(), Arc::clone(&handle.state));
    registry.insert(path.to_string(), Arc::clone(&handle));
    handle
}

fn spawn_controller_reader(path: String, state: Arc<Mutex<ControllerReaderState>>) {
    std::thread::Builder::new()
        .name(format!("arcadia-controller-reader-{}", path.rsplit('/').next().unwrap_or("js")))
        .spawn(move || controller_reader_loop(path, state))
        .ok();
}

fn controller_reader_loop(path: String, state: Arc<Mutex<ControllerReaderState>>) {
    loop {
        match std::fs::OpenOptions::new().read(true).open(&path) {
            Ok(mut file) => {
                if let Ok(mut guard) = state.lock() {
                    guard.mark_open();
                }
                let mut bytes = [0_u8; 8];
                loop {
                    match std::io::Read::read_exact(&mut file, &mut bytes) {
                        Ok(()) => {
                            let event = controller_js_event_from_bytes(bytes);
                            if let Ok(mut guard) = state.lock() {
                                guard.mark_open();
                                guard.apply_event(event);
                            }
                        }
                        Err(_) => {
                            if let Ok(mut guard) = state.lock() {
                                guard.mark_closed();
                            }
                            break;
                        }
                    }
                }
            }
            Err(_) => {
                if let Ok(mut guard) = state.lock() {
                    guard.mark_closed();
                }
            }
        }
        std::thread::sleep(Duration::from_millis(CONTROLLER_READER_REOPEN_DELAY_MS));
    }
}

fn controller_js_event_from_bytes(bytes: [u8; 8]) -> ControllerJsEvent {
    ControllerJsEvent {
        time: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        value: i16::from_le_bytes([bytes[4], bytes[5]]),
        kind: bytes[6],
        number: bytes[7],
    }
}

