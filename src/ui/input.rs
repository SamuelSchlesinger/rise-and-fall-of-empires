//! Input: the vim-style count and pending-prefix machinery, the per-mode
//! key handlers, the `:` and `/` prompts, and the mouse.

use crate::term::{Mouse, MouseKind};
use crate::ui::*;

impl Ui {
    fn remap(&self, k: Key) -> Key {
        self.keymap
            .iter()
            .find(|(a, _)| *a == k)
            .map(|(_, b)| b.clone())
            .unwrap_or(k)
    }

    fn take_count(&mut self) -> usize {
        self.count.take().unwrap_or(1).max(1)
    }

    pub(super) fn handle_key(&mut self, k: Key) -> bool {
        let k = if self.prompt == Prompt::None {
            self.remap(k)
        } else {
            k
        };
        if let Key::Mouse(m) = k {
            self.handle_mouse(m);
            return true;
        }
        if k == Key::Ctrl('c') {
            return false;
        }
        // The first-run card goes away at a touch, and never comes back.
        if self.tour {
            self.tour = false;
            mark_tour_seen();
            self.say("? for keys   : for commands   r for a recap   :q to quit");
            return true;
        }
        if self.prompt != Prompt::None {
            return self.key_prompt(k);
        }
        if self.mode == Mode::Help {
            self.key_help(&k);
            return true;
        }
        if self.mode == Mode::Fate {
            self.key_fate(&k);
            return true;
        }
        // Second key of a two-key command.
        if let Some(p) = self.pending.take() {
            let n = self.take_count();
            match (p, k) {
                ('Z', Key::Char('Z')) => return false,
                // As in vim, and as `:q!` already did: ZQ walks away without
                // writing the save file.
                ('Z', Key::Char('Q')) => {
                    self.save_path = None;
                    return false;
                }
                ('g', Key::Char('g')) => self.go_top(n),
                ('z', Key::Char('z')) | ('z', Key::Char('.')) => self.center_view(),
                ('z', Key::Char('t')) => {
                    self.view.1 = self.cursor.1;
                    self.clamp_view();
                }
                ('z', Key::Char('i')) | ('z', Key::Char('+')) => {
                    self.set_zoom(self.zoom.saturating_sub(n.max(1)));
                }
                ('z', Key::Char('o')) | ('z', Key::Char('-')) => {
                    self.set_zoom(self.zoom + n.max(1));
                }
                _ => {}
            }
            return true;
        }
        // Counts.
        if let Key::Char(c) = k {
            if c.is_ascii_digit() && !(c == '0' && self.count.is_none()) {
                let d = c as usize - '0' as usize;
                self.count = Some((self.count.unwrap_or(0) * 10 + d).min(9999));
                return true;
            }
            if matches!(c, 'g' | 'z' | 'Z') {
                self.pending = Some(c);
                return true;
            }
        }
        // Keys that work everywhere.
        let n = self.count.unwrap_or(1).max(1);
        match k {
            Key::Char(' ') => {
                self.paused = !self.paused;
                self.run_until = None;
                self.count = None;
                return true;
            }
            Key::Char('+') | Key::Char('=') | Key::Char('>') => {
                self.speed_idx = (self.speed_idx + n).min(SPEEDS.len() - 1);
                self.count = None;
                self.say(&format!("speed: {} years/sec", SPEEDS[self.speed_idx]));
                return true;
            }
            Key::Char('-') | Key::Char('_') | Key::Char('<') => {
                self.speed_idx = self.speed_idx.saturating_sub(n);
                self.count = None;
                self.say(&format!("speed: {} years/sec", SPEEDS[self.speed_idx]));
                return true;
            }
            Key::Char('.') => {
                self.count = None;
                for _ in 0..n.min(1000) {
                    self.world.tick();
                }
                self.follow_events();
                return true;
            }
            Key::Char('D') => {
                self.world.detail = self.world.detail.next();
                self.count = None;
                self.say(&format!("simulation detail: {}", self.world.detail.name()));
                return true;
            }
            Key::Char('?') | Key::F(1) => {
                self.prev_mode = self.mode;
                self.mode = Mode::Help;
                self.count = None;
                return true;
            }
            Key::Char(':') => {
                self.prompt = Prompt::Command;
                self.prompt_text.clear();
                self.count = None;
                return true;
            }
            Key::Char('/') => {
                self.prompt = Prompt::Search;
                self.prompt_text.clear();
                self.count = None;
                return true;
            }
            Key::Char('n') | Key::Char('N')
                if !matches!(self.mode, Mode::List | Mode::Chronicle) =>
            {
                self.count = None;
                self.search_step(if k == Key::Char('n') {
                    n as i32
                } else {
                    -(n as i32)
                });
                return true;
            }
            Key::Char(']') | Key::Char('[') if self.mode != Mode::List => {
                self.count = None;
                self.cycle_realm(if k == Key::Char(']') {
                    n as i32
                } else {
                    -(n as i32)
                });
                return true;
            }
            Key::Char('}') | Key::Char('{') if self.mode != Mode::List => {
                self.count = None;
                self.cycle_city(if k == Key::Char('}') {
                    n as i32
                } else {
                    -(n as i32)
                });
                return true;
            }
            Key::Char('G') => {
                self.count = None;
                self.go_bottom();
                return true;
            }
            _ => {}
        }
        match self.mode {
            Mode::Map => {
                if !self.key_map(&k) {
                    return false;
                }
            }
            Mode::List => self.key_list(&k),
            Mode::Detail => self.key_detail(&k),
            Mode::Chronicle => self.key_chronicle(&k),
            Mode::Recap => self.key_recap(&k),
            Mode::Help | Mode::Fate => {}
        }
        self.count = None;
        true
    }

    fn go_top(&mut self, _n: usize) {
        match self.mode {
            Mode::Map => self.jump_to_selected(),
            Mode::List => self.list_idx = 0,
            Mode::Detail => self.detail_scroll = 0,
            Mode::Chronicle => self.chron_scroll = usize::MAX / 2,
            Mode::Recap => self.recap_scroll = 0,
            _ => {}
        }
    }

    fn go_bottom(&mut self) {
        match self.mode {
            Mode::Map => self.jump_to_selected(),
            Mode::List => self.list_idx = usize::MAX / 2,
            Mode::Detail => self.detail_scroll = usize::MAX / 2,
            Mode::Chronicle => self.chron_scroll = 0,
            Mode::Recap => self.recap_scroll = usize::MAX / 2,
            _ => {}
        }
    }

    fn move_cursor(&mut self, dx: i32, dy: i32) {
        let tw = self.world.terrain.w as i32;
        let th = self.world.terrain.h as i32;
        let x = (self.cursor.0 as i32 + dx).clamp(0, tw - 1);
        let y = (self.cursor.1 as i32 + dy).clamp(0, th - 1);
        self.cursor = (x as usize, y as usize);
        self.clamp_view();
    }

    /// Returns false to quit.
    fn key_map(&mut self, k: &Key) -> bool {
        let n = self.count.unwrap_or(1).max(1) as i32;
        let (_, _, mw, mh) = self.map_rect();
        let tw = self.world.terrain.w;
        let th = self.world.terrain.h;
        if *k != Key::Char('q') {
            self.quit_armed = None;
        }
        match k {
            Key::Left | Key::Char('h') => self.move_cursor(-n, 0),
            Key::Right | Key::Char('l') => self.move_cursor(n, 0),
            Key::Up | Key::Char('k') => self.move_cursor(0, -n),
            Key::Down | Key::Char('j') => self.move_cursor(0, n),
            Key::Char('H') | Key::ShiftLeft => self.move_cursor(-8 * n, 0),
            Key::Char('L') | Key::ShiftRight => self.move_cursor(8 * n, 0),
            Key::Char('K') | Key::ShiftUp => self.move_cursor(0, -4 * n),
            Key::Char('J') | Key::ShiftDown => self.move_cursor(0, 4 * n),
            Key::CtrlLeft => self.move_cursor(-20 * n, 0),
            Key::CtrlRight => self.move_cursor(20 * n, 0),
            Key::CtrlUp => self.move_cursor(0, -10 * n),
            Key::CtrlDown => self.move_cursor(0, 10 * n),
            Key::PageUp | Key::Ctrl('u') => self.move_cursor(0, -(mh as i32 / 2) * n),
            Key::PageDown | Key::Ctrl('d') => self.move_cursor(0, (mh as i32 / 2) * n),
            Key::Ctrl('b') => self.move_cursor(0, -(mh as i32) * n),
            Key::Ctrl('f') => self.move_cursor(0, (mh as i32) * n),
            Key::Char('0') => self.move_cursor(-(tw as i32), 0),
            Key::Char('$') => self.move_cursor(tw as i32, 0),
            Key::Home => {
                self.cursor = (tw / 2, th / 2);
                self.center_view();
            }
            Key::Tab => self.layer = self.layer.next(),
            Key::BackTab => self.layer = self.layer.prev(),
            Key::Enter => {
                if let Some(r) = self.entity_at_cursor() {
                    self.open_detail(r);
                } else {
                    self.say("nothing here but the land");
                }
            }
            Key::Char('s') => self.selected = self.entity_at_cursor(),
            Key::Esc => {
                self.selected = None;
                self.search_results.clear();
            }
            Key::Char('e') => {
                self.prev_mode = Mode::Map;
                self.mode = Mode::List;
            }
            Key::Char('c') | Key::Char('C') => {
                self.prev_mode = Mode::Map;
                self.mode = Mode::Chronicle;
                self.chron_scroll = 0;
            }
            Key::Char('r') | Key::Char('R') => self.open_recap(None),
            Key::Char('f') | Key::Char('F') => {
                self.follow = !self.follow;
                let m = if self.follow {
                    "following major events"
                } else {
                    "cursor is free"
                };
                self.say(m);
            }
            // The same 1-3 `:log` takes. Importance 0 is not a level worth
            // cycling through: one line in the whole simulation carries it,
            // so 0 and 1 show the same feed.
            Key::Char('v') => {
                self.log_min = if self.log_min >= 3 {
                    1
                } else {
                    self.log_min + 1
                };
                self.say(&format!(
                    "event log: {} (importance {} and up)",
                    words::log_level(self.log_min),
                    self.log_min
                ));
            }
            Key::Char('x') | Key::Char('X') => self.open_fate(),
            Key::Char('t') | Key::Char('T') => {
                let st = self.stories();
                match st.first() {
                    Some((text, r)) => {
                        let (text, r) = (text.clone(), *r);
                        self.goto_ref(r);
                        self.say(&text);
                    }
                    None => self.say("a quiet moment; nothing much is happening"),
                }
            }
            Key::Char('q') => {
                if self
                    .quit_armed
                    .map(|t| t.elapsed() < Duration::from_secs(3))
                    .unwrap_or(false)
                {
                    return false;
                }
                self.quit_armed = Some(Instant::now());
                self.say("press q again to quit (or :q, ZZ)");
            }
            _ => {}
        }
        let _ = mw;
        true
    }

    fn key_list(&mut self, k: &Key) {
        let n = self.count.unwrap_or(1).max(1);
        let rows = self.list_rows();
        let len = rows.len();
        let page = self.screen.h.saturating_sub(4).max(1);
        match k {
            Key::Esc | Key::Char('q') => {
                if !self.list_filter.is_empty() {
                    self.list_filter.clear();
                } else {
                    self.mode = Mode::Map;
                }
            }
            Key::Tab | Key::Right | Key::Char('l') | Key::Char(']') => {
                self.list_tab = (self.list_tab + n) % detail::LIST_TABS.len();
                self.list_idx = 0;
                self.list_scroll = 0;
                self.list_filter.clear();
            }
            Key::BackTab | Key::Left | Key::Char('h') | Key::Char('[') => {
                self.list_tab = (self.list_tab + detail::LIST_TABS.len() * n
                    - n % detail::LIST_TABS.len())
                    % detail::LIST_TABS.len();
                self.list_idx = 0;
                self.list_scroll = 0;
                self.list_filter.clear();
            }
            Key::Up | Key::Char('k') | Key::Char('N') => {
                self.list_idx = self.list_idx.saturating_sub(n);
            }
            Key::Down | Key::Char('j') | Key::Char('n') => self.list_idx += n,
            Key::PageUp | Key::Ctrl('b') | Key::ShiftUp => {
                self.list_idx = self.list_idx.saturating_sub(page * n);
            }
            Key::PageDown | Key::Ctrl('f') | Key::ShiftDown => self.list_idx += page * n,
            Key::Ctrl('u') => self.list_idx = self.list_idx.saturating_sub(page / 2 * n),
            Key::Ctrl('d') => self.list_idx += page / 2 * n,
            Key::Home => self.list_idx = 0,
            Key::End => self.list_idx = usize::MAX / 2,
            Key::Enter => {
                if let Some((_, r)) = rows.get(self.list_idx) {
                    let r = *r;
                    self.open_detail(r);
                }
            }
            Key::Char('m') | Key::Char('s') => {
                if let Some((_, r)) = rows.get(self.list_idx) {
                    self.goto_ref(*r);
                    self.mode = Mode::Map;
                }
            }
            Key::Char('c') => self.mode = Mode::Chronicle,
            Key::Char('x') => {
                if let Some((_, r)) = rows.get(self.list_idx) {
                    self.selected = Some(*r);
                    self.open_fate();
                }
            }
            _ => {}
        }
        self.list_idx = self.list_idx.min(len.saturating_sub(1));
    }

    /// The help page scrolls, because it is longer than a small terminal.
    /// Anything that is not a way of moving through it closes it again,
    /// which keeps the old "any key returns" reflex working.
    fn key_help(&mut self, k: &Key) {
        let page = self.screen.h.saturating_sub(2).max(1);
        match k {
            Key::Down | Key::Char('j') => self.help_scroll += 1,
            Key::Up | Key::Char('k') => self.help_scroll = self.help_scroll.saturating_sub(1),
            Key::PageDown | Key::Ctrl('f') | Key::Char(' ') => self.help_scroll += page,
            Key::PageUp | Key::Ctrl('b') => {
                self.help_scroll = self.help_scroll.saturating_sub(page);
            }
            Key::Ctrl('d') => self.help_scroll += page / 2,
            Key::Ctrl('u') => self.help_scroll = self.help_scroll.saturating_sub(page / 2),
            Key::Home | Key::Char('g') => self.help_scroll = 0,
            Key::End | Key::Char('G') => self.help_scroll = usize::MAX / 2,
            _ => {
                self.help_scroll = 0;
                self.mode = self.prev_mode;
            }
        }
    }

    fn key_detail(&mut self, k: &Key) {
        let n = self.count.unwrap_or(1).max(1);
        let r = match self.selected {
            Some(r) => r,
            None => {
                self.mode = Mode::Map;
                return;
            }
        };
        let page = self.screen.h.saturating_sub(2).max(1);
        match k {
            Key::Esc | Key::Char('q') => {
                self.mode = self.prev_mode;
                if self.mode == Mode::Detail {
                    self.mode = Mode::Map;
                }
            }
            Key::Backspace => {
                if let Some(prev) = self.back.pop() {
                    self.selected = Some(prev);
                    self.detail_scroll = 0;
                } else {
                    self.mode = self.prev_mode;
                }
            }
            Key::Up | Key::Char('k') => self.detail_scroll = self.detail_scroll.saturating_sub(n),
            Key::Down | Key::Char('j') => self.detail_scroll += n,
            Key::PageUp | Key::Ctrl('b') | Key::ShiftUp => {
                self.detail_scroll = self.detail_scroll.saturating_sub(page * n);
            }
            Key::PageDown | Key::Ctrl('f') | Key::ShiftDown => self.detail_scroll += page * n,
            Key::Ctrl('u') => self.detail_scroll = self.detail_scroll.saturating_sub(page / 2 * n),
            Key::Ctrl('d') => self.detail_scroll += page / 2 * n,
            Key::Home => self.detail_scroll = 0,
            Key::End => self.detail_scroll = usize::MAX / 2,
            Key::Char('m') => {
                self.goto_ref(r);
                self.mode = Mode::Map;
            }
            Key::Char('e') => self.mode = Mode::List,
            Key::Char('x') => {
                if let Ref::Polity(_) = r {
                    self.prev_mode = Mode::Detail;
                    self.mode = Mode::Fate;
                }
            }
            Key::Enter => {
                self.goto_ref(r);
                self.mode = Mode::Map;
            }
            Key::Char(c) => {
                if let Some(link) = detail::follow_link(&self.world, r, *c) {
                    self.open_detail(link);
                }
            }
            _ => {}
        }
    }

    fn key_chronicle(&mut self, k: &Key) {
        let n = self.count.unwrap_or(1).max(1);
        let page = self.screen.h.saturating_sub(2).max(1);
        match k {
            Key::Esc | Key::Char('q') => {
                if !self.chron_filter.is_empty() {
                    self.chron_filter.clear();
                } else {
                    self.mode = self.prev_mode;
                }
            }
            Key::Up | Key::Char('k') => self.chron_scroll += n,
            Key::Down | Key::Char('j') => self.chron_scroll = self.chron_scroll.saturating_sub(n),
            Key::PageUp | Key::Ctrl('b') | Key::ShiftUp => self.chron_scroll += page * n,
            Key::PageDown | Key::Ctrl('f') | Key::ShiftDown => {
                self.chron_scroll = self.chron_scroll.saturating_sub(page * n);
            }
            Key::Ctrl('u') => self.chron_scroll += page / 2 * n,
            Key::Ctrl('d') => self.chron_scroll = self.chron_scroll.saturating_sub(page / 2 * n),
            Key::End => self.chron_scroll = 0,
            Key::Home => self.chron_scroll = usize::MAX / 2,
            Key::Char('f') | Key::Char('v') => self.chron_min = (self.chron_min + 1) % 4,
            Key::Char('e') => self.mode = Mode::List,
            _ => {}
        }
    }

    fn key_recap(&mut self, k: &Key) {
        let n = self.count.unwrap_or(1).max(1);
        let page = self.screen.h.saturating_sub(2).max(1);
        match k {
            Key::Esc | Key::Char('q') | Key::Backspace => self.mode = self.prev_mode,
            Key::Up | Key::Char('k') => self.recap_scroll = self.recap_scroll.saturating_sub(n),
            Key::Down | Key::Char('j') => self.recap_scroll += n,
            Key::PageUp | Key::Ctrl('b') | Key::ShiftUp => {
                self.recap_scroll = self.recap_scroll.saturating_sub(page * n);
            }
            Key::PageDown | Key::Ctrl('f') | Key::ShiftDown => self.recap_scroll += page * n,
            Key::Ctrl('u') => self.recap_scroll = self.recap_scroll.saturating_sub(page / 2 * n),
            Key::Ctrl('d') => self.recap_scroll += page / 2 * n,
            Key::Home => self.recap_scroll = 0,
            Key::End => self.recap_scroll = usize::MAX / 2,
            Key::Char('c') => self.mode = Mode::Chronicle,
            Key::Char('e') => self.mode = Mode::List,
            Key::Enter | Key::Char('m') => self.mode = Mode::Map,
            _ => {}
        }
    }

    fn key_fate(&mut self, k: &Key) {
        let p = match self.selected {
            Some(Ref::Polity(p)) => p,
            _ => {
                self.mode = self.prev_mode;
                return;
            }
        };
        match k {
            Key::Esc | Key::Char('x') | Key::Char('q') => self.mode = self.prev_mode,
            Key::Char(c) if ('1'..='6').contains(c) => {
                self.choose_fate(p, *c as usize - '1' as usize);
            }
            _ => {}
        }
    }

    // -- prompts: `:` commands and `/` search ------------------------------

    fn key_prompt(&mut self, k: Key) -> bool {
        match k {
            Key::Esc => {
                if self.prompt == Prompt::Search {
                    match self.mode {
                        Mode::List => self.list_filter.clear(),
                        Mode::Chronicle => self.chron_filter.clear(),
                        _ => {}
                    }
                }
                self.prompt = Prompt::None;
            }
            Key::Enter => {
                let text = std::mem::take(&mut self.prompt_text);
                let kind = self.prompt;
                self.prompt = Prompt::None;
                match kind {
                    Prompt::Command => return self.run_command(text.trim()),
                    Prompt::Search => self.submit_search(&text),
                    Prompt::None => {}
                }
            }
            Key::Backspace => {
                if self.prompt_text.pop().is_none() {
                    self.prompt = Prompt::None;
                }
                self.live_filter();
            }
            Key::Ctrl('u') => {
                self.prompt_text.clear();
                self.live_filter();
            }
            Key::Paste(text) => {
                self.prompt_text
                    .extend(text.chars().filter(|c| !c.is_control()));
                self.live_filter();
            }
            Key::Char(c) => {
                self.prompt_text.push(c);
                self.live_filter();
            }
            _ => {}
        }
        true
    }

    /// In lists and the chronicle, a search filters rows as you type.
    fn live_filter(&mut self) {
        if self.prompt != Prompt::Search {
            return;
        }
        match self.mode {
            Mode::List => {
                self.list_filter = self.prompt_text.clone();
                self.list_idx = 0;
            }
            Mode::Chronicle => {
                self.chron_filter = self.prompt_text.clone();
                self.chron_scroll = 0;
            }
            _ => {}
        }
    }

    pub(super) fn submit_search(&mut self, text: &str) {
        match self.mode {
            Mode::List => self.list_filter = text.to_string(),
            Mode::Chronicle => self.chron_filter = text.to_string(),
            _ => {
                if text.trim().is_empty() {
                    return;
                }
                self.search_results = self.search(text);
                self.search_idx = 0;
                if self.search_results.is_empty() {
                    self.say(&format!("no match for \"{}\"", text));
                } else {
                    let r = self.search_results[0];
                    self.goto_ref(r);
                    if self.mode == Mode::Detail {
                        self.open_detail(r);
                    }
                    self.say(&format!(
                        "{} of {} matches: {}",
                        1,
                        self.search_results.len(),
                        detail::entity_name(&self.world, r)
                    ));
                }
            }
        }
    }

    fn search_step(&mut self, delta: i32) {
        if self.search_results.is_empty() {
            self.say("no search yet (press / to search)");
            return;
        }
        let len = self.search_results.len() as i32;
        self.search_idx = ((self.search_idx as i32 + delta).rem_euclid(len)) as usize;
        let r = self.search_results[self.search_idx];
        self.goto_ref(r);
        if self.mode == Mode::Detail {
            self.open_detail(r);
        }
        self.say(&format!(
            "{} of {} matches: {}",
            self.search_idx + 1,
            len,
            detail::entity_name(&self.world, r)
        ));
    }

    // -- mouse ---------------------------------------------------------------

    fn handle_mouse(&mut self, m: Mouse) {
        let double = matches!(self.last_click, Some((x, y, t)) if x == m.x && y == m.y && t.elapsed() < Duration::from_millis(500));
        if let MouseKind::Press(_) = m.kind {
            self.last_click = Some((m.x, m.y, Instant::now()));
        }
        if self.prompt != Prompt::None {
            return;
        }
        match self.mode {
            Mode::Help => {
                if let MouseKind::Press(_) = m.kind {
                    self.mode = self.prev_mode;
                }
            }
            Mode::Recap => match m.kind {
                MouseKind::WheelUp => self.recap_scroll = self.recap_scroll.saturating_sub(3),
                MouseKind::WheelDown => self.recap_scroll += 3,
                MouseKind::Press(_) => self.mode = self.prev_mode,
                _ => {}
            },
            Mode::Fate => {
                if let MouseKind::Press(_) = m.kind {
                    let (fx, fy, fw, fh) = self.fate_rect;
                    if m.x >= fx && m.x < fx + fw && m.y >= fy && m.y < fy + fh {
                        let row = m.y.saturating_sub(fy + 1);
                        if (2..8).contains(&row) {
                            if let Some(Ref::Polity(p)) = self.selected {
                                self.choose_fate(p, row - 2);
                            }
                        }
                    } else {
                        self.mode = self.prev_mode;
                    }
                }
            }
            Mode::Map => {
                let (_, _, mw, mh) = self.map_rect();
                match m.kind {
                    MouseKind::WheelUp => self.move_cursor(0, -3 * self.zoom as i32),
                    MouseKind::WheelDown => self.move_cursor(0, 3 * self.zoom as i32),
                    MouseKind::Press(button) => {
                        let (padx, pady) = self.view_pad();
                        if m.x < mw && m.y < mh && m.x >= padx && m.y >= pady {
                            let z = self.zoom;
                            let cx = self.view.0 + (m.x - padx) * z + z / 2;
                            let cy = self.view.1 + (m.y - pady) * z + z / 2;
                            if cx < self.world.terrain.w && cy < self.world.terrain.h {
                                let same = self.cursor == (cx, cy);
                                self.cursor = (cx, cy);
                                self.clamp_view();
                                if button == 2 || (same && double) {
                                    if let Some(r) = self.entity_at_cursor() {
                                        self.open_detail(r);
                                    }
                                } else {
                                    self.selected = self.entity_at_cursor();
                                }
                            }
                        } else if let Some(&(_, p)) = self
                            .power_rows
                            .iter()
                            .find(|&&(y, _)| y == m.y)
                            .filter(|_| m.x >= mw)
                        {
                            self.goto_ref(Ref::Polity(p));
                            if double || button == 2 {
                                self.open_detail(Ref::Polity(p));
                            }
                        } else if let Some(&(_, r)) = self
                            .story_rows
                            .iter()
                            .find(|&&(y, _)| y == m.y)
                            .filter(|_| m.x >= mw)
                        {
                            self.goto_ref(r);
                            if double || button == 2 {
                                self.open_detail(r);
                            }
                        } else if let Some(&(_, e)) = self.log_rows.iter().find(|&&(y, _)| y == m.y)
                        {
                            self.jump_to_event(e);
                        }
                    }
                    _ => {}
                }
            }
            Mode::List => match m.kind {
                MouseKind::WheelUp => self.list_idx = self.list_idx.saturating_sub(3),
                MouseKind::WheelDown => self.list_idx += 3,
                MouseKind::Press(button) => {
                    if m.y == 0 {
                        // Tab bar: pick the tab whose label spans this column.
                        let mut x = 1;
                        for (i, t) in detail::LIST_TABS.iter().enumerate() {
                            let w = t.chars().count() + 2;
                            if m.x >= x && m.x < x + w {
                                self.list_tab = i;
                                self.list_idx = 0;
                                self.list_filter.clear();
                            }
                            x += w + 1;
                        }
                    } else if m.y >= self.list_y0 {
                        let row = m.y - self.list_y0 + self.list_scroll;
                        let rows = self.list_rows();
                        if row < rows.len() {
                            let r = rows[row].1;
                            // The right button opens at once; the left needs
                            // the row to be the selected one already.
                            if button == 2 || (row == self.list_idx && double) {
                                self.open_detail(r);
                            }
                            self.list_idx = row;
                        }
                    }
                }
                _ => {}
            },
            Mode::Detail => match m.kind {
                MouseKind::WheelUp => self.detail_scroll = self.detail_scroll.saturating_sub(3),
                MouseKind::WheelDown => self.detail_scroll += 3,
                MouseKind::Press(2) => {
                    if let Some(prev) = self.back.pop() {
                        self.selected = Some(prev);
                        self.detail_scroll = 0;
                    } else {
                        self.mode = self.prev_mode;
                    }
                }
                _ => {}
            },
            Mode::Chronicle => match m.kind {
                MouseKind::WheelUp => self.chron_scroll += 3,
                MouseKind::WheelDown => self.chron_scroll = self.chron_scroll.saturating_sub(3),
                MouseKind::Press(_) => {
                    if let Some(&(_, e)) = self.chron_rows.iter().find(|&&(y, _)| y == m.y) {
                        self.jump_to_event(e);
                        self.mode = Mode::Map;
                    }
                }
                _ => {}
            },
        }
    }
}
