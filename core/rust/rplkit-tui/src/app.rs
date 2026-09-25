//! app — state machine TUI (murni data + transisi, tanpa IO terminal).
//!
//! Rendering di `views` (Fase 4b), event loop di entri `run` (Fase 5).
//! Semua transisi keyboard diuji tanpa terminal.

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rplkit_core::sys as coresys;
use rplkit_runtime::{chain, history, modules, registry};

/// Layar TUI. `ModuleDetail` membawa nama modul.
#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Main,
    Tools,
    Exec,
    Modules,
    ModuleDetail(String),
    Palette,
    Logs,
    Help,
}

/// Satu baris daftar tool (siap tampil, sudah terfilter enabled).
#[derive(Debug, Clone)]
pub struct ToolRow {
    pub name: String,
    pub desc: String,
    pub runtime: String,
    pub module: String,
    pub group: String,
}

/// Satu baris daftar modul.
#[derive(Debug, Clone)]
pub struct ModuleRow {
    pub name: String,
    pub version: String,
    pub runtime: String,
    pub description: String,
    pub enabled: bool,
    pub tool_count: usize,
}

/// Status eksekusi terakhir di Exec view.
#[derive(Debug, Clone, Default)]
pub struct ExecState {
    pub tool: String,
    pub args_line: String,
    pub output: String,
    pub code: Option<i32>,
    pub by: String,
    pub ms: u128,
    pub copied: bool,
}

pub struct App {
    pub repo_root: PathBuf,
    pub view: View,
    pub tools: Vec<ToolRow>,
    pub modules: Vec<ModuleRow>,
    pub cursor: usize,
    pub tools_query: String,
    pub searching: bool,
    pub palette_open: bool,
    pub palette_query: String,
    pub palette_cursor: usize,
    pub exec: ExecState,
    pub session_log: Vec<history::RunRecord>,
    pub recent: Vec<history::RunRecord>,
    pub sys: Option<coresys::Snapshot>,
    pub message: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(repo_root: &Path) -> Self {
        let mut app = Self {
            repo_root: repo_root.to_path_buf(),
            view: View::Main,
            tools: Vec::new(),
            modules: Vec::new(),
            cursor: 0,
            tools_query: String::new(),
            searching: false,
            palette_open: false,
            palette_query: String::new(),
            palette_cursor: 0,
            exec: ExecState::default(),
            session_log: Vec::new(),
            recent: history::load(repo_root),
            sys: None,
            message: None,
            should_quit: false,
        };
        app.refresh();
        app
    }

    /// Muat ulang tools + modules dari disk (setelah enable/disable).
    pub fn refresh(&mut self) {
        let tools = registry::discover_enabled(&self.repo_root);
        self.tools = tools
            .iter()
            .map(|t| ToolRow {
                name: t.name.clone(),
                desc: t.description.clone(),
                runtime: if t.py_entry.is_some() {
                    "python".into()
                } else if t.ts_entry.is_some() {
                    "typescript".into()
                } else {
                    "native".into()
                },
                module: t.module.clone(),
                group: t.group_name().to_string(),
            })
            .collect();
        let descs = modules::load_descriptors(&self.repo_root);
        let overlay = modules::load_overlay(&self.repo_root);
        self.modules = descs
            .iter()
            .map(|d| ModuleRow {
                name: d.name.clone(),
                version: d.version.clone(),
                runtime: d.runtime.clone(),
                description: d.description.clone(),
                enabled: modules::is_enabled(d, &overlay, None),
                tool_count: tools.iter().filter(|t| t.module == d.name).count(),
            })
            .collect();
        self.cursor = 0;
        self.recent = history::load(&self.repo_root);
    }

    pub fn tick_sys(&mut self) {
        self.sys = Some(coresys::snapshot());
    }

    // -- navigasi generik -------------------------------------------------

    pub fn move_cursor(&mut self, delta: isize, len: usize) {
        if len == 0 {
            self.cursor = 0;
            return;
        }
        let c = self.cursor as isize;
        self.cursor = c.rem_euclid(len as isize).wrapping_add(delta).rem_euclid(len as isize) as usize;
    }

    pub fn switch_view(&mut self, view: View) {
        self.view = view;
        self.cursor = 0;
        self.searching = false;
        self.message = None;
    }

    // -- filter tools (substring; fuzzy penuh di palette via nucleo) --------

    pub fn visible_tools(&self) -> Vec<usize> {
        let q = self.tools_query.to_lowercase();
        let mut vis: Vec<usize> = self
            .tools
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                q.is_empty()
                    || t.name.to_lowercase().contains(&q)
                    || t.desc.to_lowercase().contains(&q)
                    || t.group.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect();
        // Urutan tampil: grup lalu nama (stabil untuk kursor + Enter).
        vis.sort_by(|&a, &b| {
            self.tools[a]
                .group
                .cmp(&self.tools[b].group)
                .then(self.tools[a].name.cmp(&self.tools[b].name))
        });
        vis
    }

    // -- palette (fuzzy nucleo atas tool + aksi) -----------------------------

    pub fn palette_items(&self) -> Vec<(String, String)> {
        // (label, kind) — kind: "run:<tool>" | "action:<id>".
        let mut items = Vec::new();
        let mut matcher = nucleo::Matcher::new(nucleo::Config::DEFAULT);
        let q = self.palette_query.trim();
        let mut scored: Vec<(u32, String, String)> = Vec::new();
        let mut buf = Vec::new();
        for t in &self.tools {
            let label = format!("{} — {}", t.name, t.desc);
            let score = if q.is_empty() {
                Some(0)
            } else {
                nucleo::pattern::Pattern::parse(
                    q,
                    nucleo::pattern::CaseMatching::Ignore,
                    nucleo::pattern::Normalization::Smart,
                )
                .score(nucleo::Utf32Str::new(&label, &mut buf), &mut matcher)
            };
            if let Some(s) = score {
                scored.push((s, label, format!("run:{}", t.name)));
            }
        }
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, label, kind) in scored {
            items.push((label, kind));
        }
        for (label, id) in [
            ("Create module", "action:create-module"),
            ("Reload modules", "action:reload"),
            ("Open settings", "action:settings"),
        ] {
            if q.is_empty() || label.to_lowercase().contains(&q.to_lowercase()) {
                items.push((label.to_string(), id.to_string()));
            }
        }
        items
    }

    // -- aksi -----------------------------------------------------------------

    /// Toggle modul di bawah kursor (Modules view). Return pesan status.
    pub fn toggle_module_at(&mut self, idx: usize) -> String {
        let (name, target) = match self.modules.get(idx) {
            Some(m) => (m.name.clone(), !m.enabled),
            None => return "no module".into(),
        };
        match modules::set_module_enabled(&self.repo_root, &name, target) {
            Ok(()) => {
                self.refresh();
                format!("module {name} {}", if target { "enabled" } else { "disabled" })
            }
            Err(e) => format!("error: {e}"),
        }
    }

    /// Jalankan tool terpilih via chain capture; catat history + sesi.
    pub fn run_tool(&mut self, name: &str, args: &[String]) -> (i32, String) {
        let tools = registry::discover_enabled(&self.repo_root);
        let Some(tool) = registry::find(&tools, name) else {
            return (1, format!("unknown or disabled tool: {name}"));
        };
        let c = chain::run_tool_captured(&self.repo_root, tool, args);
        let rec = history::RunRecord {
            tool: name.to_string(),
            args: args.to_vec(),
            code: c.code,
            ms: c.ms,
            by: c.executed_by.to_string(),
            at: history::now_unix(),
        };
        history::append(&self.repo_root, rec.clone());
        self.session_log.insert(0, rec);
        self.recent = history::load(&self.repo_root);
        self.exec = ExecState {
            tool: name.to_string(),
            args_line: args.join(" "),
            output: c.output.clone(),
            code: Some(c.code),
            by: c.executed_by.to_string(),
            ms: c.ms,
            copied: false,
        };
        (c.code, c.output)
    }

    // -- keyboard ---------------------------------------------------------------

    /// Return true bila event loop harus berhenti.
    pub fn on_key(&mut self, key: KeyEvent) -> bool {
        // Palette menangkap semua kecuali Esc/Ctrl+K toggle.
        if self.palette_open {
            return self.palette_key(key);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('k') | KeyCode::Char('K'))
        {
            self.palette_open = true;
            self.palette_query.clear();
            self.palette_cursor = 0;
            return false;
        }
        // Input search Tools view.
        if self.searching && self.view == View::Tools {
            match key.code {
                KeyCode::Esc => self.searching = false,
                KeyCode::Backspace => {
                    self.tools_query.pop();
                    self.cursor = 0;
                }
                KeyCode::Char(c) => {
                    self.tools_query.push(c);
                    self.cursor = 0;
                }
                _ => {}
            }
            return false;
        }
        // Input argumen Exec view.
        // (Menyimpang dari mock: 'r'/'c' polos konflik dengan mengetik argumen,
        // jadi run-again = Ctrl+R, copy = Ctrl+Y. Enter = run.)
        if self.view == View::Exec && !matches!(key.code, KeyCode::Esc) {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        let args = split_args(&self.exec.args_line);
                        let tool = self.exec.tool.clone();
                        if !tool.is_empty() {
                            self.run_tool(&tool, &args);
                        }
                        return false;
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.exec.copied = crate::copy_osc52(&self.exec.output);
                        if !self.exec.copied {
                            self.message = Some("copy unavailable in this terminal".into());
                        }
                        return false;
                    }
                    _ => {}
                }
            }
            match key.code {
                KeyCode::Backspace => {
                    self.exec.args_line.pop();
                }
                KeyCode::Char(c) => self.exec.args_line.push(c),
                KeyCode::Enter => {
                    let args = split_args(&self.exec.args_line);
                    let tool = self.exec.tool.clone();
                    if !tool.is_empty() {
                        self.run_tool(&tool, &args);
                    }
                }
                _ => {}
            }
            if !matches!(key.code, KeyCode::Char('1'..='4') | KeyCode::Char('?') | KeyCode::Char('q'))
            {
                return false;
            }
        }
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return true;
            }
            KeyCode::Char('1') => self.switch_view(View::Tools),
            KeyCode::Char('2') => self.switch_view(View::Modules),
            KeyCode::Char('3') => self.switch_view(View::Logs),
            KeyCode::Char('?') => self.switch_view(View::Help),
            KeyCode::Esc => self.switch_view(View::Main),
            KeyCode::Up => self.move_cursor(-1, self.current_len()),
            KeyCode::Down => self.move_cursor(1, self.current_len()),
            KeyCode::Char('/') if self.view == View::Tools => self.searching = true,
            KeyCode::Enter => self.activate(),
            KeyCode::Char(' ') if self.view == View::Modules => {
                let msg = self.toggle_module_at(self.cursor);
                self.message = Some(msg);
            }
            _ => {}
        }
        false
    }

    fn current_len(&self) -> usize {
        match self.view {
            View::Tools => self.visible_tools().len(),
            View::Modules => self.modules.len(),
            View::Logs => self.session_log.len(),
            _ => 0,
        }
    }

    fn activate(&mut self) {
        match self.view.clone() {
            View::Tools => {
                let vis = self.visible_tools();
                if let Some(&i) = vis.get(self.cursor) {
                    let name = self.tools[i].name.clone();
                    self.exec = ExecState {
                        tool: name,
                        ..ExecState::default()
                    };
                    self.switch_view(View::Exec);
                }
            }
            View::Modules => {
                if let Some(m) = self.modules.get(self.cursor).cloned() {
                    self.switch_view(View::ModuleDetail(m.name));
                }
            }
            View::ModuleDetail(_) => self.switch_view(View::Modules),
            _ => {}
        }
    }

    fn palette_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.palette_open = false;
                self.palette_query.clear();
            }
            KeyCode::Backspace => {
                self.palette_query.pop();
                self.palette_cursor = 0;
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.palette_query.push(c);
                self.palette_cursor = 0;
            }
            KeyCode::Up => {
                let n = self.palette_items().len();
                if n > 0 && self.palette_cursor > 0 {
                    self.palette_cursor -= 1;
                }
            }
            KeyCode::Down => {
                let n = self.palette_items().len();
                if n > 0 && self.palette_cursor + 1 < n {
                    self.palette_cursor += 1;
                }
            }
            KeyCode::Enter => {
                let items = self.palette_items();
                if let Some((_, kind)) = items.get(self.palette_cursor) {
                    let kind = kind.clone();
                    self.palette_open = false;
                    self.palette_query.clear();
                    self.apply_palette(&kind);
                }
            }
            _ => {}
        }
        false
    }

    fn apply_palette(&mut self, kind: &str) {
        if let Some(tool) = kind.strip_prefix("run:") {
            let name = tool.to_string();
            self.exec = ExecState { tool: name, ..ExecState::default() };
            self.switch_view(View::Exec);
        } else {
            self.message = Some(match kind {
                "action:reload" => {
                    self.refresh();
                    "modules reloaded".to_string()
                }
                _ => format!("action queued: {kind}"),
            });
        }
    }
}

/// Split argumen gaya shell (dipakai Exec view).
pub fn split_args(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_s = false;
    let mut in_d = false;
    let mut esc = false;
    let mut has = false;
    for c in line.chars() {
        if esc {
            cur.push(c);
            esc = false;
            has = true;
        } else if c == '\\' && !in_s {
            esc = true;
        } else if c == '\'' && !in_d {
            in_s = !in_s;
            has = true;
        } else if c == '"' && !in_s {
            in_d = !in_d;
            has = true;
        } else if (c == ' ' || c == '\t') && !in_s && !in_d {
            if has {
                out.push(std::mem::take(&mut cur));
                has = false;
            }
        } else {
            cur.push(c);
            has = true;
        }
    }
    if has {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
    }

    #[test]
    fn loads_tools_and_modules() {
        let app = App::new(&test_repo());
        assert!(app.tools.iter().any(|t| t.name == "calculator"));
        assert!(app.tools.iter().any(|t| t.name == "system-info"));
        assert!(app.modules.iter().any(|m| m.name == "system-tools"));
        let school = app.modules.iter().find(|m| m.name == "school-tools").unwrap();
        assert!(!school.enabled);
    }

    #[test]
    fn navigation_keys() {
        let mut app = App::new(&test_repo());
        app.on_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::empty()));
        assert_eq!(app.view, View::Tools);
        let n = app.visible_tools().len();
        assert!(n > 0);
        app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(app.cursor, 1 % n);
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(app.view, View::Main);
    }

    #[test]
    fn search_filters() {
        let mut app = App::new(&test_repo());
        app.switch_view(View::Tools);
        for c in "disk".chars() {
            app.searching = true;
            app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
        }
        let vis = app.visible_tools();
        assert_eq!(vis.len(), 1);
        assert_eq!(app.tools[vis[0]].name, "disk");
        // Filter grup: "system-tools" mencakup 5 tool modul itu.
        app.tools_query = "system-tools".into();
        assert_eq!(app.visible_tools().len(), 5);
    }

    #[test]
    fn palette_fuzzy_finds_tool() {
        let app = App::new(&test_repo());
        let mut app2 = app;
        app2.palette_query = "sysinfo".into();
        let items = app2.palette_items();
        assert!(items.iter().any(|(_, k)| k == "run:system-info"), "{items:?}");
    }

    #[test]
    fn palette_ctrl_k_toggle() {
        let mut app = App::new(&test_repo());
        app.on_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert!(app.palette_open);
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(!app.palette_open);
    }

    #[test]
    fn split_args_vectors() {
        assert_eq!(split_args("a \"b c\" d"), vec!["a", "b c", "d"]);
        assert_eq!(split_args(""), Vec::<String>::new());
    }

    #[test]
    fn run_tool_records_history() {
        // Fixture terisolasi: history tertulis di temp, bukan repo asli.
        let repo = std::env::temp_dir().join(format!("rplkit-tuihist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        let dir = repo.join("tools").join("t");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("tool.json"),
            "{\"name\": \"t\", \"version\": \"1\", \"description\": \"t\", \
             \"module\": \"m\", \"exec\": [\"native\"], \"native\": {\"op\": \"calc\"}}",
        )
        .unwrap();
        std::fs::create_dir_all(repo.join("modules")).unwrap();
        std::fs::write(
            repo.join("modules").join("m.module.json"),
            "{\"name\": \"m\", \"version\": \"1\", \"description\": \"d\", \"runtime\": \"rust\"}",
        )
        .unwrap();
        let mut app = App::new(&repo);
        let (code, out) = app.run_tool("t", &["6 * 7".to_string()]);
        assert_eq!(code, 0);
        assert!(out.contains("42"), "{out}");
        assert_eq!(app.exec.by, "native");
        assert_eq!(app.session_log.len(), 1);
        assert_eq!(history::load(&repo).len(), 1);
        let _ = std::fs::remove_dir_all(&repo);
    }
}
