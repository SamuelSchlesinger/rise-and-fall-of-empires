//! In-app reading and a short tutorial using the real interface handlers.
//! The player guide is embedded from the same Markdown shipped with releases.
//! Tutorial input is limited to the action being taught, and skipping restores
//! the caller's time/follow settings without loading or saving a world.

use super::*;

const GUIDE: &str = include_str!("../../docs/GUIDE.md");

/// Strip Markdown presentation while preserving inline code and link labels.
fn plain(text: &str) -> String {
    let mut text = text.to_string();
    while let Some(end) = text.find("](") {
        let Some(start) = text[..end].rfind('[') else {
            break;
        };
        let Some(close) = text[end + 2..].find(')') else {
            break;
        };
        let label = text[start + 1..end].to_string();
        text.replace_range(start..end + 3 + close, &label);
    }
    let mut code = false;
    text.chars()
        .filter(|&c| {
            if c == '`' {
                code = !code;
                false
            } else {
                c != '*' || code
            }
        })
        .collect()
}

/// Fold paragraphs and turn table rows into label/value entries for a narrow
/// terminal. This handles the small Markdown subset used by our own guide.
pub(super) fn guide_lines(width: usize) -> Vec<String> {
    let width = width.max(4);
    let mut lines = Vec::new();
    let mut paragraph = String::new();
    let mut code = false;
    let flush = |paragraph: &mut String, lines: &mut Vec<String>| {
        if !paragraph.is_empty() {
            lines.extend(fold(&plain(paragraph), width));
            paragraph.clear();
        }
    };
    for line in GUIDE.lines() {
        if line.starts_with("```") {
            flush(&mut paragraph, &mut lines);
            code = !code;
        } else if code {
            lines.extend(fold(line, width));
        } else if line.is_empty() {
            flush(&mut paragraph, &mut lines);
            lines.push(String::new());
        } else if line.starts_with('#') {
            flush(&mut paragraph, &mut lines);
            lines.extend(fold(
                &plain(line.trim_start_matches('#').trim()).to_uppercase(),
                width,
            ));
        } else if line.starts_with('|') {
            flush(&mut paragraph, &mut lines);
            if line.chars().all(|c| "|-: ".contains(c)) {
                continue;
            }
            let fields: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
            lines.extend(fold(&plain(&fields.join(": ")), width));
        } else {
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(line);
        }
    }
    flush(&mut paragraph, &mut lines);
    lines
}

/// Paths and command names can exceed a narrow terminal's width. Split those
/// words too so the right edge never silently drops part of an instruction.
fn fold(text: &str, width: usize) -> Vec<String> {
    term::wrap(text, width)
        .into_iter()
        .flat_map(|line| {
            if line.is_empty() {
                return vec![String::new()];
            }
            line.chars()
                .collect::<Vec<_>>()
                .chunks(width)
                .map(|chunk| chunk.iter().collect())
                .collect()
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Step {
    Resume,
    Pause,
    Move,
    Layer,
    Zoom,
    Browse,
    Back,
    Reach,
    Done,
}

const STEPS: [Step; 9] = [
    Step::Resume,
    Step::Pause,
    Step::Move,
    Step::Layer,
    Step::Zoom,
    Step::Browse,
    Step::Back,
    Step::Reach,
    Step::Done,
];

impl Step {
    fn lesson(self) -> (&'static str, &'static str) {
        match self {
            Self::Resume => ("Time", "Press Space to start time. This world develops on its own; you choose when to watch or look closer."),
            Self::Pause => ("Pause", "Press Space to pause again. Pausing gives you time to explore without missing events."),
            Self::Move => ("Explore", "Move with an arrow key or h j k l. The sidebar describes the place under your cursor."),
            Self::Layer => ("Map layers", "Press Tab to change what the map shows: who holds what, the peoples, the land, how they live, and what is moving. Press \\ to step through the maps within one. The legend under the map explains its colours."),
            Self::Zoom => ("Zoom", "Type zo to zoom out. More of the world fits on screen. Type zi when you want to look closer again."),
            Self::Browse => ("Lists", "Press e to browse the world's realms, cities, peoples and more. In a list, Tab changes category and Enter opens an item."),
            Self::Back => ("Return", "Press Esc to return to the map. On a detail page, bracketed letters follow links and Backspace retraces them."),
            // The one verb the player has. A tutorial that teaches the
            // interface and never the game leaves somebody knowing how to
            // read a world they have no part in.
            Self::Reach => ("Reach in", "Press x. Whatever the cursor is on — a realm, a town, a person, or open country — can be leaned on, and each of the six acts costs devotion, which your people give you. The number is in the status bar. Esc closes the menu without spending anything."),
            Self::Done => ("Ready", "Press Enter to finish. :bind changes who you serve. :w saves your world. ? opens help; press p there for the player guide. Reopen this tutorial with :tutorial."),
        }
    }
}

pub(super) struct Tutorial {
    step: Step,
    was_paused: bool,
    was_following: bool,
    run_until: Option<i32>,
}

impl Ui {
    pub(super) fn open_guide(&mut self) {
        if !matches!(self.mode, Mode::Help | Mode::Guide) {
            self.prev_mode = self.mode;
        }
        self.mode = Mode::Guide;
        self.guide_scroll = 0;
    }

    pub(super) fn start_tutorial(&mut self) {
        if self.tutorial.is_some() {
            self.stop_tutorial();
        }
        self.tutorial = Some(Tutorial {
            step: Step::Resume,
            was_paused: self.paused,
            was_following: self.follow,
            run_until: self.run_until.take(),
        });
        self.tour = false;
        self.mode = Mode::Map;
        self.prev_mode = Mode::Map;
        self.paused = true;
        self.follow = false;
        self.layer = Layer::Political;
        self.set_zoom(1);
        self.pending = None;
        self.count = None;
        self.prompt = Prompt::None;
        self.quit_armed = None;
        self.acc = 0.0;
    }

    pub(super) fn stop_tutorial(&mut self) {
        if let Some(tutorial) = self.tutorial.take() {
            self.paused = tutorial.was_paused;
            self.follow = tutorial.was_following;
            self.run_until = tutorial.run_until;
        }
        self.mode = Mode::Map;
        self.prev_mode = Mode::Map;
        self.pending = None;
        self.count = None;
        self.acc = 0.0;
        self.last = Instant::now();
        self.say("? for help   :guide to read   :tutorial to practice again");
    }

    pub(super) fn tutorial_key(&mut self, key: Key) -> bool {
        let step = self.tutorial.as_ref().unwrap().step;
        if key == Key::Ctrl('c') {
            return false;
        }
        // Esc closes an open menu before it means anything else, whatever
        // step the lesson has reached: a player who opens the Hand of Fate
        // and backs out of it has not asked to abandon the tutorial.
        if key == Key::Esc && matches!(self.mode, Mode::Fate | Mode::Covenant) {
            self.mode = self.prev_mode;
            return true;
        }
        // Otherwise Esc is the lesson in the Back step and the way out
        // everywhere else.
        if key == Key::Ctrl('g') || (key == Key::Esc && step != Step::Back) {
            self.stop_tutorial();
            return true;
        }
        if step == Step::Done {
            if key == Key::Enter {
                self.stop_tutorial();
            }
            return true;
        }
        let allowed = match step {
            Step::Resume | Step::Pause => key == Key::Char(' '),
            Step::Move => matches!(
                key,
                Key::Up | Key::Down | Key::Left | Key::Right | Key::Char('h' | 'j' | 'k' | 'l')
            ),
            Step::Layer => key == Key::Tab,
            Step::Zoom => matches!(key, Key::Char('z' | 'o' | 'i')),
            Step::Browse => key == Key::Char('e'),
            Step::Back => key == Key::Esc,
            Step::Reach => matches!(key, Key::Char('x' | 'X')),
            Step::Done => false,
        };
        if !allowed {
            return true;
        }
        let cursor = self.cursor;
        let layer = self.layer;
        self.handle_key_normal(key);
        let complete = match step {
            Step::Resume => !self.paused,
            Step::Pause => self.paused,
            Step::Move => self.cursor != cursor,
            // That the layer changed, not that it became any particular
            // one. Naming a destination here tied the tutorial to what Tab
            // happened to land on, and the step stalled silently the day
            // Tab began stepping between families instead.
            Step::Layer => self.layer != layer,
            Step::Zoom => self.zoom > 1,
            Step::Browse => self.mode == Mode::List,
            Step::Back => self.mode == Mode::Map,
            // Pressing the key is the lesson. Whether the menu opened
            // depends on what happens to be under the cursor, and a
            // tutorial that can stall on an empty sea is worse than one
            // that teaches a keystroke.
            Step::Reach => true,
            Step::Done => false,
        };
        if complete {
            let next = STEPS[step as usize + 1];
            self.tutorial.as_mut().unwrap().step = next;
            // The Reach lesson needs something to reach. Wherever the
            // cursor wandered to in the Explore step, it may be open ocean,
            // and a tutorial whose instruction does nothing is worse than
            // no tutorial.
            if next == Step::Reach {
                if let Some(p) = self
                    .world
                    .alive_polities
                    .iter()
                    .copied()
                    .max_by_key(|&p| self.world.polities[p].cells)
                {
                    self.goto_ref(crate::sim::chronicle::Ref::Polity(p));
                    self.selected = Some(crate::sim::chronicle::Ref::Polity(p));
                }
            }
        }
        true
    }

    pub(super) fn render_tutorial(&mut self) {
        let step = self.tutorial.as_ref().unwrap().step;
        let (title, body) = step.lesson();
        let width = self.screen.w.saturating_sub(2).max(4);
        let lines = term::wrap(body, width);
        let height = (lines.len() + 2).min(self.screen.h.saturating_sub(1));
        let y = self.screen.h.saturating_sub(height + 1);
        let bg = Rgb(26, 32, 45);
        let fg = Rgb(225, 225, 235);
        self.screen.fill(
            Rect::new(0, y, self.screen.w, height),
            ' ',
            Style::new(fg, bg),
        );
        self.screen.text_clip(
            1,
            y,
            &format!("Tutorial {}/{}: {}", step as usize + 1, STEPS.len(), title),
            width,
            Style::attr(Rgb(255, 220, 120), bg, BOLD),
        );
        for (i, line) in lines.iter().take(height.saturating_sub(2)).enumerate() {
            self.screen
                .text_clip(1, y + i + 1, line, width, Style::new(fg, bg));
        }
        let skip = if step == Step::Back {
            "Ctrl-g skip"
        } else {
            "Esc / Ctrl-g skip"
        };
        self.screen.text_clip(
            1,
            y + height.saturating_sub(1),
            skip,
            width,
            Style::new(Rgb(160, 210, 160), bg),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Detail;

    fn ui() -> Ui {
        // Ticked, because several of these lessons are about things a world
        // has to contain: a realm to reach into, a list with rows in it.
        let mut w = World::new(7, 40, 20, Detail::Medium);
        for _ in 0..80 {
            w.tick();
        }
        Ui::new(w, false, false, 80, 24)
    }

    #[test]
    fn tutorial_uses_real_controls_and_restores_time_settings() {
        let mut ui = ui();
        ui.paused = false;
        ui.follow = true;
        ui.run_until = Some(500);
        let year = ui.world.year;
        ui.start_tutorial();
        assert!(ui.paused);
        for key in [Key::Char(' '), Key::Char(' ')] {
            assert!(ui.handle_key(key));
        }
        let direction = if ui.cursor.0 > 0 {
            Key::Left
        } else {
            Key::Right
        };
        for key in [
            direction,
            Key::Tab,
            Key::Char('z'),
            Key::Char('o'),
            Key::Char('e'),
        ] {
            assert!(ui.handle_key(key));
        }
        assert_eq!(ui.mode, Mode::List);
        assert!(ui.handle_key(Key::Esc));
        assert_eq!(ui.tutorial.as_ref().unwrap().step, Step::Reach);
        // The lesson that is about the game rather than the interface: x
        // opens the menu, and Esc closes it without ending the tutorial.
        assert!(ui.handle_key(Key::Char('x')));
        assert_eq!(ui.mode, Mode::Fate);
        assert_eq!(ui.tutorial.as_ref().unwrap().step, Step::Done);
        assert!(ui.handle_key(Key::Esc));
        assert_eq!(ui.mode, Mode::Map);
        assert!(
            ui.tutorial.is_some(),
            "Esc ended the tutorial from the menu"
        );
        assert!(ui.handle_key(Key::Enter));
        assert!(ui.tutorial.is_none());
        assert!(!ui.paused);
        assert!(ui.follow);
        assert_eq!(ui.run_until, Some(500));
        assert_eq!(ui.mode, Mode::Map);
        // Tab moved the layer along; which one it landed on is the layer
        // machinery's business, not the tutorial's.
        assert_ne!(ui.layer, Layer::Political);
        assert_eq!(ui.zoom, 2);
        assert_eq!(ui.world.year, year);
        assert!(ui.save_path.is_none());
    }

    #[test]
    fn tutorial_can_be_skipped_at_every_step_even_with_remapped_keys() {
        let mut ui = ui();
        ui.paused = true;
        ui.follow = false;
        ui.keymap.push((Key::Ctrl('g'), Key::Char('x')));
        for step in STEPS {
            ui.start_tutorial();
            ui.tutorial.as_mut().unwrap().step = step;
            assert!(ui.handle_key(Key::Char(':')));
            assert_eq!(ui.prompt, Prompt::None);
            assert!(ui.handle_key(Key::Ctrl('g')));
            assert!(ui.tutorial.is_none());
            assert!(ui.paused);
            assert!(!ui.follow);
        }
    }

    #[test]
    fn guide_opens_from_help_and_returns_to_the_original_screen() {
        let mut ui = ui();
        ui.mode = Mode::List;
        ui.handle_key(Key::Char('?'));
        ui.handle_key(Key::Char('p'));
        assert_eq!(ui.mode, Mode::Guide);
        ui.handle_key(Key::PageDown);
        assert!(ui.guide_scroll > 0);
        ui.handle_key(Key::Esc);
        assert_eq!(ui.mode, Mode::List);
    }

    #[test]
    fn guide_preserves_content_without_markdown_links() {
        let lines = guide_lines(60).join("\n");
        assert!(lines.contains("HOW THE WORLD WORKS"));
        assert!(lines.contains("COMMAND-LINE OPTIONS"));
        assert!(lines.contains(":autosave N"));
        assert!(!lines.contains("](../"));
        assert_eq!(
            plain("**Use** `*` and [the guide](docs/GUIDE.md)."),
            "Use * and the guide."
        );
        for width in [17, 20, 37, 77] {
            let lines = guide_lines(width);
            assert!(lines.iter().all(|line| line.chars().count() <= width));
            assert!(lines
                .join("")
                .contains("~/.local/share/empires/world-SEED.rfe"));
        }
    }

    #[test]
    fn narrow_tutorials_keep_the_action_and_exit_visible() {
        let mut ui = ui();
        ui.start_tutorial();
        for (cols, rows) in [(20, 5), (40, 10), (80, 24)] {
            ui.screen.resize(cols, rows);
            ui.compose();
            let output: String = (0..rows)
                .flat_map(|y| (0..cols).map(move |x| (x, y)))
                .map(|(x, y)| ui.screen.cell(x, y).ch)
                .collect();
            assert!(
                output.contains("Space"),
                "{}x{} lost the action",
                cols,
                rows
            );
            assert!(
                output.contains("Ctrl-g skip"),
                "{}x{} lost the exit",
                cols,
                rows
            );
        }
    }
}
