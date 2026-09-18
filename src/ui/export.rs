//! Writing a world out so it can be read somewhere else.
//!
//! Everything the interface knows is on a screen that scrolls away. A world
//! run for five thousand years is a thing people want to keep, argue about
//! and show to somebody who does not have the game — and the only way out
//! was a save file, which is no use to anybody without this binary.
//!
//! Three shapes, because three different questions get asked of a world:
//!
//! * **The chronicle as Markdown**, grouped by century, which is the form a
//!   history wants to be read in.
//! * **The map as HTML**, one coloured cell per character, which is the only
//!   way to show somebody what a world *looks* like.
//! * **The tables as CSV**, because the question "which of my realms is
//!   going broke" deserves a spreadsheet, and because a list page that can
//!   be filtered is halfway to one already.

use super::{detail, Ui};
use crate::term::Rgb;

impl Ui {
    /// Write the world out in whatever shape `what` names.
    ///
    /// Returns what to tell the player, which is either where it went or
    /// why it did not.
    pub(crate) fn export(&mut self, what: &str, path: &str) -> String {
        let (kind, arg) = match what.split_once(char::is_whitespace) {
            Some((k, rest)) => (k, rest.trim()),
            None => (what, ""),
        };
        let kind = if kind.is_empty() { "chronicle" } else { kind };
        let body = match kind {
            "chronicle" | "history" => self.chronicle_markdown(),
            "map" => self.map_html(),
            "timeline" => self.timeline_markdown(),
            "series" => self.series_csv(),
            "realms" | "wealth" | "cities" | "roads" | "persons" | "wars" | "houses" => {
                match self.table_csv(kind) {
                    Some(csv) => csv,
                    None => return format!("no {} page to write out", kind),
                }
            }
            _ => {
                return "usage: :export chronicle|map|timeline|series|realms|wealth|cities|roads|persons|wars|houses [PATH]"
                    .into()
            }
        };
        let path = if !path.is_empty() {
            path.to_string()
        } else if !arg.is_empty() {
            arg.to_string()
        } else {
            let ext = match kind {
                "map" => "html",
                "chronicle" | "history" | "timeline" => "md",
                _ => "csv",
            };
            // Named for the seed and the year, because those two identify a
            // world exactly and a world has no name of its own stored
            // anywhere — genesis coins one for its prose and keeps it
            // nowhere.
            format!(
                "world-{:x}-{}-{}.{}",
                self.world.seed, kind, self.world.year, ext
            )
        };
        match std::fs::write(&path, body) {
            Ok(()) => format!("wrote {}", path),
            Err(e) => format!("could not write {}: {}", path, e),
        }
    }

    /// The chronicle, grouped by century, as Markdown.
    fn chronicle_markdown(&self) -> String {
        let w = &self.world;
        let mut out = String::new();
        out.push_str(&format!("# A history of {} years\n\n", w.year));
        out.push_str(&format!(
            "From seed {:#x}, as it stood in year {}. {} events are \
             recorded; {} have been forgotten.\n\n",
            w.seed,
            w.year,
            w.chronicle.len(),
            w.chronicle.dropped
        ));
        let mut century = i32::MIN;
        for e in &w.chronicle.events {
            // Only what the reader has asked to see, so an export matches
            // the chronicle they have been reading rather than quietly
            // holding more.
            if e.importance < self.chron_min || self.muted.contains(&e.kind) {
                continue;
            }
            let c = e.year.div_euclid(100) * 100;
            if c != century {
                century = c;
                out.push_str(&format!("\n## Years {}–{}\n\n", c.max(0), c + 99));
            }
            out.push_str(&format!("- **{}** — {}\n", e.year, e.text));
        }
        out
    }

    /// The world's own record of itself, as CSV: one row per decade, one
    /// column per series.
    ///
    /// The columns come from `history::SERIES`, the same table the chart
    /// page draws from, so a column cannot appear on one and not the other.
    fn series_csv(&self) -> String {
        use crate::sim::history::SERIES;
        let mut out = String::from("year");
        for (name, _) in SERIES {
            out.push(',');
            out.push_str(name);
        }
        out.push('\n');
        for s in &self.world.history.samples {
            out.push_str(&s.year.to_string());
            for (_, read) in SERIES {
                out.push_str(&format!(",{:.4}", read(s)));
            }
            out.push('\n');
        }
        out
    }

    /// The rise and fall of every realm, as Markdown.
    ///
    /// Drawn from the same `list_rows` the Timeline page uses, so the file
    /// and the screen cannot disagree about who descended from whom — and
    /// the reader's filter applies, since it is `Ui::list_rows` that runs
    /// the query.
    fn timeline_markdown(&self) -> String {
        let was = self.list_tab;
        let tab = detail::LIST_TABS
            .iter()
            .position(|t| t.eq_ignore_ascii_case("timeline"))
            .unwrap_or(was);
        let (rows, total) = detail::list_rows(&self.world, tab);
        let mut out = format!("# The rise and fall of {} realms\n\n", total);
        out.push_str(&format!(
            "As it stood in year {}, from seed {:#x}. Each bar runs from a \
             realm's founding to its fall across the whole of history; \
             successor states are indented under the realm they broke away \
             from.\n\n```\n",
            self.world.year, self.world.seed
        ));
        out.push_str(detail::list_header(tab).trim_end());
        out.push('\n');
        for (text, _) in &rows {
            out.push_str(text.trim_end());
            out.push('\n');
        }
        out.push_str("```\n");
        out
    }

    /// The map as an HTML page, one coloured cell per character.
    ///
    /// Composed from the screen the player is looking at rather than
    /// re-rendered, so what comes out is what they saw, legend and all.
    fn map_html(&mut self) -> String {
        self.compose();
        let mut html = String::from("<!doctype html><meta charset=utf-8><title>");
        html.push_str(&format!("A world of {} years", self.world.year));
        html.push_str(
            "</title><style>body{background:#0b0b0e;margin:0;padding:12px;\
             font:13px/1.15 ui-monospace,Menlo,Consolas,monospace}\
             pre{margin:0}</style><pre>",
        );
        let mut last: Option<(Rgb, Rgb, u8)> = None;
        for y in 0..self.screen.h {
            for x in 0..self.screen.w {
                let c = self.screen.cell(x, y);
                let st = (c.fg, c.bg, c.attr);
                if last != Some(st) {
                    if last.is_some() {
                        html.push_str("</span>");
                    }
                    html.push_str(&format!(
                        "<span style=\"color:rgb({},{},{});background:rgb({},{},{}){}\">",
                        c.fg.0,
                        c.fg.1,
                        c.fg.2,
                        c.bg.0,
                        c.bg.1,
                        c.bg.2,
                        if c.attr & crate::term::BOLD != 0 {
                            ";font-weight:bold"
                        } else {
                            ""
                        }
                    ));
                    last = Some(st);
                }
                match c.ch {
                    '<' => html.push_str("&lt;"),
                    '>' => html.push_str("&gt;"),
                    '&' => html.push_str("&amp;"),
                    ch => html.push(ch),
                }
            }
            html.push('\n');
        }
        html.push_str("</span></pre>");
        html
    }

    /// A list page as CSV, with the numbers the filter can ask about as
    /// their own columns.
    ///
    /// The rendered rows are padded for a terminal and useless in a
    /// spreadsheet, so the name is taken from the row and everything else
    /// from `query::value_of` — which means a column exists for exactly the
    /// quantities the page advertises, and nothing has to be kept in step
    /// by hand.
    fn table_csv(&self, kind: &str) -> Option<String> {
        let tab = detail::LIST_TABS
            .iter()
            .position(|t| t.eq_ignore_ascii_case(kind))?;
        let (rows, _) = detail::list_rows(&self.world, tab);
        let fields: Vec<&str> = crate::ui::query::fields_for(tab)
            .split_whitespace()
            .collect();
        let mut out = String::from("name");
        for f in &fields {
            out.push(',');
            out.push_str(f);
        }
        out.push('\n');
        for (text, r) in &rows {
            // The first run of two or more spaces ends the name in every
            // one of these rows, which is how they are laid out.
            let name = text.trim().split("  ").next().unwrap_or("").trim();
            out.push('"');
            out.push_str(&name.replace('"', "\"\""));
            out.push('"');
            for f in &fields {
                out.push(',');
                if let Some(v) = crate::ui::query::value_of(&self.world, *r, f) {
                    out.push_str(&format!("{:.3}", v));
                }
            }
            out.push('\n');
        }
        Some(out)
    }
}
