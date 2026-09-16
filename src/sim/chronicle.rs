//! The chronicle: every notable thing that happens, as structured events
//! with prose already rendered. Indexed by the entities involved so that
//! any polity, city, person or school can show its own history on demand.

use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Ref {
    Polity(usize),
    City(usize),
    Culture(usize),
    Person(usize),
    School(usize),
    War(usize),
    Race(usize),
    Feature(usize),
    Artifact(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventKind {
    Genesis,
    Founding,
    Politics,
    Death,
    War,
    Battle,
    Peace,
    Disaster,
    Magic,
    Culture,
    Era,
    Wonder,
    Discovery,
    Person,
}

impl EventKind {
    pub fn all() -> [EventKind; 14] {
        [
            EventKind::Genesis,
            EventKind::Founding,
            EventKind::Politics,
            EventKind::Death,
            EventKind::War,
            EventKind::Battle,
            EventKind::Peace,
            EventKind::Disaster,
            EventKind::Magic,
            EventKind::Culture,
            EventKind::Era,
            EventKind::Wonder,
            EventKind::Discovery,
            EventKind::Person,
        ]
    }
    pub fn name(self) -> &'static str {
        match self {
            EventKind::Genesis => "genesis",
            EventKind::Founding => "founding",
            EventKind::Politics => "politics",
            EventKind::Death => "death",
            EventKind::War => "war",
            EventKind::Battle => "battle",
            EventKind::Peace => "peace",
            EventKind::Disaster => "disaster",
            EventKind::Magic => "magic",
            EventKind::Culture => "culture",
            EventKind::Era => "era",
            EventKind::Wonder => "wonder",
            EventKind::Discovery => "discovery",
            EventKind::Person => "person",
        }
    }
    pub fn from_name(s: &str) -> Option<EventKind> {
        let s = s.to_lowercase();
        EventKind::all()
            .into_iter()
            .find(|k| k.name().starts_with(&s) && !s.is_empty())
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    pub year: i32,
    pub importance: u8,
    pub kind: EventKind,
    pub refs: Vec<Ref>,
    pub loc: Option<usize>,
    pub text: String,
}

#[derive(Default)]
pub struct Chronicle {
    pub events: Vec<Event>,
    by_ref: BTreeMap<Ref, Vec<usize>>,
}

impl Chronicle {
    pub fn push(&mut self, ev: Event) -> usize {
        let id = self.events.len();
        for r in &ev.refs {
            self.by_ref.entry(*r).or_default().push(id);
        }
        self.events.push(ev);
        id
    }

    /// Rebuild the index after loading events.
    pub fn from_events(events: Vec<Event>) -> Chronicle {
        let mut c = Chronicle::default();
        for e in events {
            c.push(e);
        }
        c
    }

    pub fn for_ref(&self, r: Ref) -> &[usize] {
        self.by_ref.get(&r).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}
