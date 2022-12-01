use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    str::FromStr,
    time::{Duration, Instant},
};

use crossbeam_channel::{unbounded, Receiver, Sender};
use livesplit_hotkey::{Hook, KeyCode};

#[derive(Debug)]
pub enum Error {
    HookCreate,

    ActionAlreadyExists,
    ActionDoesNotExist(MapType),
    KeyNotMapped,

    MappedKeyMissingInReverseLookup,

    BadKeyCodeName,
    CannotRegisterHotkey(livesplit_hotkey::Error),
    CannotUnregisterHotkey(livesplit_hotkey::Error),
}

#[derive(Debug)]
pub enum MapType {
    Actions,
    ActionMapping,
    ReverseLookup,
}

type Result<T> = std::result::Result<T, Error>;

/// Stores all actions associated with a key sequence along with the last-pressed time for each key.
#[derive(Debug, Clone)]
pub struct ActionMapping {
    actions: Vec<String>,
    keys: HashMap<KeyCode, Instant>,
}

impl ActionMapping {
    fn new(keys: &[KeyCode]) -> Self {
        let mut hm = HashMap::new();
        let offset = Duration::from_secs(60);
        for key in keys.iter() {
            hm.insert(key.clone(), Instant::now() - offset);
        }

        ActionMapping {
            actions: vec![],
            keys: hm,
        }
    }

    /// Update the last pressed time for a given keycode.
    /// Panics if the key does not exist as this should not be possible.
    fn press_key(&mut self, key: &KeyCode) {
        match self.keys.get_mut(key) {
            Some(time) => *time = Instant::now(),
            None => unreachable!(),
        }
    }

    /// Iterates through every single key's timestamp and compares it to the passed
    /// `min_elapsed_time`. If all timestamps are less tan the `min_elapsed_time`,
    /// then the Action is considered to be pressed.
    fn is_pressed(&self, min_elapsed_time: &Duration) -> bool {
        for time in self.keys.values() {
            if time.elapsed() > *min_elapsed_time {
                return false;
            }
        }

        true
    }

    /// Adds an action to be emitted when all hotkeys are pressed.
    fn add_action(&mut self, action: &String) -> Result<()> {
        if self.actions.contains(action) {
            return Err(Error::ActionAlreadyExists);
        }

        self.actions.push(action.clone());

        Ok(())
    }

    /// Removes an action to be emitted when all hotkeys are pressed.
    fn remove_action(&mut self, action: &String) -> Result<()> {
        if !self.actions.contains(action) {
            return Err(Error::ActionDoesNotExist(MapType::ActionMapping));
        }

        self.actions.retain(|a| a != action);

        Ok(())
    }
}

/// Listens for hotkeys being pressed. If a registered sequence of keys is pressed within a minimum amount of time,
/// then the actions associated with the key sequence is emitted.
pub struct HotkeyListener {
    hook: Hook,

    actions: HashMap<u64, ActionMapping>,
    reverse_lookup: HashMap<KeyCode, Vec<u64>>,

    min_elapsed_time: Duration,

    callback_sender: Sender<KeyCode>,
    callback_receiver: Receiver<KeyCode>,

    listener_sender: Sender<String>,
}

impl HotkeyListener {
    /// Creates a new instance of `HotkeyListener`. This operation _can_ fail.
    pub fn new(listener_sender: Sender<String>) -> Result<Self> {
        let hook = match Hook::new() {
            Ok(h) => h,
            Err(e) => {
                eprintln!("{e}");
                return Err(Error::HookCreate);
            }
        };

        let (sender, receiver) = unbounded::<KeyCode>();

        Ok(HotkeyListener {
            hook: hook,

            actions: HashMap::new(),
            reverse_lookup: HashMap::new(),

            min_elapsed_time: Duration::from_secs_f32(0.2), // TODO hardcoded value?

            callback_sender: sender,
            callback_receiver: receiver,

            listener_sender: listener_sender,
        })
    }

    /// Registers an action by name and key sequence. The key sequence is hashed and that hash is used to store
    /// action names.
    ///
    /// For every key associated with the action, a reverse lookup is used (key -> action) for quick access.
    pub fn register_action(&mut self, action_name: &String, keys: &[String]) -> Result<()> {
        let (key_codes, key_codes_hash) = match string_slice_to_vec_and_hash(keys) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        match self.actions.get_mut(&key_codes_hash) {
            Some(am) => match am.add_action(action_name) {
                Ok(_) => {}
                Err(e) => return Err(e),
            },
            None => {
                let mut am = ActionMapping::new(key_codes.as_slice());
                am.add_action(action_name).unwrap();
                self.actions.insert(key_codes_hash, am);
            }
        }

        for key in key_codes.iter() {
            match self.reverse_lookup.get_mut(key) {
                Some(v) => {
                    if !v.contains(&key_codes_hash) {
                        v.push(key_codes_hash);
                    }
                }
                None => {
                    let sender = self.callback_sender.clone();
                    let key = key.clone();
                    match self.hook.register(key, move || match sender.send(key) {
                        Ok(_) => {}
                        Err(e) => eprintln!("{e}"),
                    }) {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(Error::CannotRegisterHotkey(e));
                        }
                    }
                    self.reverse_lookup.insert(key, vec![key_codes_hash]);
                }
            }
        }

        Ok(())
    }

    /// Safely removes an action + key sequence without accidentally removing other action's hotkeys.
    /// If no more actions depend on a certain key, the hook for that key is unregistered.
    pub fn unregister_action(&mut self, action_name: &String, keys: &[String]) -> Result<()> {
        let (key_codes, key_codes_hash) = match string_slice_to_vec_and_hash(keys) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let mut is_empty_hash = false;

        match self.actions.get_mut(&key_codes_hash) {
            Some(am) => match am.remove_action(action_name) {
                Ok(_) => {
                    if am.actions.len() < 1 {
                        is_empty_hash = true;
                    }
                }
                Err(e) => return Err(e),
            },
            None => return Err(Error::ActionDoesNotExist(MapType::Actions)),
        }

        if !is_empty_hash {
            return Ok(());
        }

        let mut empty_keys: Vec<KeyCode> = vec![];

        match self.actions.remove(&key_codes_hash) {
            Some(_) => {}
            None => unreachable!(),
        }

        for key in key_codes.iter() {
            match self.reverse_lookup.get_mut(key) {
                Some(v) => {
                    v.retain(|hash| hash != &key_codes_hash);
                    if v.is_empty() {
                        empty_keys.push(*key);
                    }
                }
                None => unreachable!(),
            }
        }

        for key in empty_keys.iter() {
            match self.reverse_lookup.remove(key) {
                Some(_) => match self.hook.unregister(*key) {
                    Ok(_) => {}
                    Err(e) => return Err(Error::CannotUnregisterHotkey(e)),
                },
                None => unreachable!(),
            }
        }

        Ok(())
    }

    // TODO maybe we should clear the channel? Clearing the channel might infinitely loop though
    /// Checks if any actions have been triggered. Needs to be polled at regular intervals
    /// or else the receivers might grow infinitely large or the senders might block infinitely.
    pub fn poll(&mut self) {
        if self.callback_receiver.is_empty() {
            return;
        }

        match self.callback_receiver.recv() {
            Ok(key) => {
                if !self.reverse_lookup.contains_key(&key) {
                    return;
                }

                let vec = match self.reverse_lookup.get(&key) {
                    Some(v) => v,
                    None => {
                        return;
                    }
                };

                for hash in vec.iter() {
                    match self.actions.get_mut(&hash) {
                        Some(am) => {
                            am.press_key(&key);
                            if am.is_pressed(&self.min_elapsed_time) {
                                for action_name in am.actions.iter() {
                                    match self.listener_sender.send(action_name.clone()) {
                                        Ok(_) => {}
                                        Err(e) => eprintln!("{e}"),
                                    }
                                }
                            }
                        }
                        None => unreachable!(),
                    }
                }
            }
            Err(e) => eprintln!("{e}"),
        }
    }

    /// Returns the minimum elapsed time as an `f32` in seconds.
    pub fn get_min_elapsed_time(&self) -> f32 {
        self.min_elapsed_time.as_secs_f32()
    }

    /// Converts an `f32` into a `Duration`. Treats the `f32` as seconds.
    pub fn set_min_elapsed_time(&mut self, min_elapsed_time: f32) {
        self.min_elapsed_time = Duration::from_secs_f32(min_elapsed_time);
    }

    /// Iterates through all actions and returns a non-repeating `Vec` of all registered actions.
    ///
    /// The `Vec` is initially unsorted but is sorted in order to remove duplicates.
    pub fn get_action_names(&self) -> Vec<String> {
        let mut r = self
            .actions
            .values()
            .into_iter()
            .flat_map(|am| am.actions.clone())
            .collect::<Vec<String>>();

        r.sort_unstable();
        r.dedup();

        r
    }

    /// Iterates through all reverse lookup keys and returns their names as a `Vec`.
    ///
    /// Names are _not_ sorted.
    pub fn get_key_names(&self) -> Vec<String> {
        self.reverse_lookup
            .keys()
            .into_iter()
            .map(|k| k.as_str().to_string())
            .collect::<Vec<String>>()
    }
}

/// Converts a `String` slice to a `Vec<String>` and then takes the hash of that `Vec`.
/// Sorts the keys beforehand to ensure ordering doesn't impact the hash.
fn string_slice_to_vec_and_hash(keys: &[String]) -> Result<(Vec<KeyCode>, u64)> {
    let mut keys = keys.to_vec();
    keys.sort();

    let mut key_codes = vec![];
    for key in keys.iter() {
        match KeyCode::from_str(key) {
            Ok(k) => key_codes.push(k),
            Err(_) => return Err(Error::BadKeyCodeName),
        };
    }

    let key_codes_hash = get_hash(&key_codes);

    Ok((key_codes, key_codes_hash))
}

/// Gets the hash of some data using a new hasher.
fn get_hash<T: Hash>(data: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);

    hasher.finish()
}
