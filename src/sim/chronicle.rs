//! The chronicle: every notable thing that happens, as structured events
//! with prose already rendered. Indexed by the entities involved so that
//! any polity, city, person or school can show its own history on demand.

use std::collections::BTreeMap;

/// A pointer at something in the world: what an event is *about*, and what
/// the interface follows when a line is opened.
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
    House(usize),
}

/// What sort of thing happened, for colouring and for `:mute`.
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
    /// Every kind, in a fixed order.
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
    /// The lowercase name `:mute` and the filters use.
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
    /// The first kind whose name starts with `s`, ignoring case.
    pub fn from_name(s: &str) -> Option<EventKind> {
        let s = s.to_lowercase();
        EventKind::all()
            .into_iter()
            .find(|k| k.name().starts_with(&s) && !s.is_empty())
    }
}

/// One line of history, with its prose already written.
#[derive(Clone, Debug)]
pub struct Event {
    /// The year it happened.
    pub year: i32,
    /// 0 for a footnote, 3 for an age-defining event.
    pub importance: u8,
    /// What sort of thing it was.
    pub kind: EventKind,
    /// Everything it is about, in the order the prose mentions them.
    pub refs: Vec<Ref>,
    /// The cell to jump to, if it happened anywhere in particular.
    pub loc: Option<usize>,
    /// The sentence itself.
    pub text: String,
    /// Whether the watcher's own hand caused this.
    ///
    /// A world of fifty thousand events swallows six of them without trace,
    /// and an act you cannot find afterwards is an act that did not feel
    /// like one. Marked here rather than remembered as a list of indices,
    /// because compaction drops events and renumbers everything after them.
    pub by_fate: bool,
}

/// Every event of a world's life, indexed by what each one is about.
#[derive(Default)]
pub struct Chronicle {
    /// The events themselves, oldest first.
    pub events: Vec<Event>,
    by_ref: BTreeMap<Ref, Vec<usize>>,
    /// How many events compaction has thrown away over the world's life.
    pub dropped: usize,
    /// Length at which compaction is worth trying again. It rises when a
    /// compaction finds nothing left to drop, so a chronicle made entirely of
    /// great events does not get re-scanned every year.
    next_compact: usize,
}

impl Chronicle {
    /// Record an event and index it. Returns its id.
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

    /// The ids of every event about `r`, oldest first.
    pub fn for_ref(&self, r: Ref) -> &[usize] {
        self.by_ref
            .get(&r)
            .map(std::vec::Vec::as_slice)
            .unwrap_or(&[])
    }

    /// How many events are still kept (compaction drops the least of them).
    pub fn len(&self) -> usize {
        self.events.len()
    }

    fn reindex(&mut self) {
        self.by_ref.clear();
        for (id, ev) in self.events.iter().enumerate() {
            for r in &ev.refs {
                self.by_ref.entry(*r).or_default().push(id);
            }
        }
    }

    /// Keep the chronicle from growing without bound. Over `cap` events, the
    /// oldest goes first, and the least important of the old before the rest:
    /// footnotes, then ordinary business, then the things a realm remembers.
    /// Only importance 3 — the handful of age-defining events, five places in
    /// the whole simulation — is kept whatever happens.
    ///
    /// Event positions shift, so the by-ref index is rebuilt; `for_ref` and
    /// the entity pages keep working. It trims well below the cap so that it
    /// runs rarely rather than every year.
    ///
    /// The sweep used to stop at importance 1, which meant that on a long
    /// game the cap was a fiction: a large map at the eightieth century held
    /// four hundred thousand events against a cap of sixty thousand, because
    /// almost nothing left was droppable. Worse than the memory was the
    /// thrashing — every few thousand events it rescanned the whole vector,
    /// failed to free anything, and rebuilt a by-ref index of a million
    /// entries for nothing. That is why the back-off below is proportional:
    /// a compaction that cannot reach its target waits longer before trying
    /// again, so even a chronicle that is all age-defining events costs a
    /// constant amount of work per event rather than a growing one.
    pub fn compact(&mut self, cap: usize) {
        if cap == 0 || self.events.len() <= cap || self.events.len() < self.next_compact {
            return;
        }
        let target = cap - cap / 8;
        let mut excess = self.events.len() - target;
        let mut doomed = vec![false; self.events.len()];
        for level in 0..=2u8 {
            if excess == 0 {
                break;
            }
            for (i, ev) in self.events.iter().enumerate() {
                if excess == 0 {
                    break;
                }
                if !doomed[i] && ev.importance == level {
                    doomed[i] = true;
                    excess -= 1;
                }
            }
        }
        let before = self.events.len();
        let mut i = 0;
        self.events.retain(|_| {
            let keep = !doomed[i];
            i += 1;
            keep
        });
        self.dropped += before - self.events.len();
        // Ordinarily the next sweep is due once the cap has been earned back.
        // When this one could not reach its target there is nothing to earn
        // back, so wait for the chronicle to grow by an eighth of itself
        // instead: a futile scan costs what the chronicle costs, and doing it
        // at geometric intervals is what keeps that from compounding.
        let step = if excess == 0 {
            cap / 16 + 1
        } else {
            self.events.len() / 8 + cap / 16 + 1
        };
        self.next_compact = self.events.len() + step;
        self.reindex();
    }
}
