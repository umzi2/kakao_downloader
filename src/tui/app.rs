use std::{ cell::Cell, sync::Arc };

use crossterm::event::{ KeyCode, KeyEvent, KeyModifiers };
use ratatui::DefaultTerminal;
use tokio::{ sync::mpsc::{ UnboundedSender, unbounded_channel }, task::JoinHandle };

use crate::{
    config::{ CONFIG_PATH, Config, auto_tolerance },
    download::{ PARALLEL_PAGES, Progress, SliceOptions },
    error::{ Context, Result },
    kakao::{ ClientEvent, KakaoClient, chapters::chapter::Chapter, url::parse_title_id },
    retry::ATTEMPTS_LABEL,
    tui::{ input::TextInput, tasks::{ self, Message, RetryAction }, ui },
};

pub const PER_PAGE: usize = 18;

pub const TAB_COUNT: usize = 5;

pub const SLICE_TOGGLE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Cookies,
    Title,
    Chapters,
    Slice,
    Confirm,
    Run,
}

impl Screen {
    pub fn tab(self) -> usize {
        match self {
            Self::Cookies => 0,
            Self::Title => 1,
            Self::Chapters => 2,
            Self::Slice | Self::Confirm => 3,
            Self::Run => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Problem {
    pub message: String,
    pub retry: Option<RetryAction>,
    pub auth: bool,
}

#[derive(Debug, Default)]
pub struct RunState {
    pub stage: String,
    pub log_done: usize,
    pub detail: String,
    pub percent: u16,
    pub log: Vec<String>,
    pub running: bool,
    pub summary: Option<String>,
}

pub struct App {
    pub config: Config,
    pub screen: Screen,
    pub cookie: TextInput,
    pub title_input: TextInput,
    pub chapters: Vec<Chapter>,
    pub search: TextInput,
    pub search_active: bool,
    pub selected: usize,
    pub page: usize,
    pub title_id: Option<usize>,
    pub chapter_total: usize,
    pub chapter: Option<Chapter>,
    pub slice_inputs: [TextInput; 4],
    pub slice_focus: usize,
    pub run: RunState,
    pub notice: Option<String>,
    pub problem: Option<Problem>,
    pub busy: bool,
    pub quit: bool,
    client: Option<Arc<KakaoClient>>,
    messages: UnboundedSender<Message>,
    run_task: Option<JoinHandle<()>>,
    pending_retry: Option<RetryAction>,
    previous_screen: Screen,
    list_height: Cell<usize>,
}

pub async fn run(terminal: &mut DefaultTerminal) -> Result<()> {
    let (messages_tx, mut messages_rx) = unbounded_channel();
    let (input_tx, mut input_rx) = unbounded_channel();
    crate::tui::spawn_input_reader(input_tx);

    let mut app = App::new(messages_tx).await;

    while !app.quit {
        terminal.draw(|frame| ui::render(frame, &app)).context("отрисовка кадра")?;

        tokio::select! {
            event = input_rx.recv() => {
                match event {
                    Some(crossterm::event::Event::Key(key)) => app.on_key(key).await,
                    Some(crossterm::event::Event::Paste(text)) => app.on_paste(&text),
                    Some(_) => {}
                    None => break,
                }
            }
            message = messages_rx.recv() => {
                match message {
                    Some(message) => app.on_message(message).await,
                    None => break,
                }
            }
        }
    }

    Ok(())
}

impl App {
    pub async fn new(messages: UnboundedSender<Message>) -> Self {
        let mut app = Self {
            config: Config::default(),
            screen: Screen::Cookies,
            cookie: TextInput::default(),
            title_input: TextInput::default(),
            chapters: Vec::new(),
            search: TextInput::default(),
            search_active: false,
            selected: 0,
            page: 0,
            title_id: None,
            chapter_total: 0,
            chapter: None,
            slice_inputs: [
                TextInput::default(),
                TextInput::default(),
                TextInput::default(),
                TextInput::default(),
            ],
            slice_focus: 0,
            run: RunState::default(),
            notice: None,
            problem: None,
            busy: false,
            quit: false,
            client: None,
            messages,
            run_task: None,
            pending_retry: None,
            previous_screen: Screen::Title,
            list_height: Cell::new(PER_PAGE),
        };

        app.sync_slice_inputs();

        if !Config::exists() {
            app.screen = Screen::Cookies;
            app.notice = Some(
                format!("{CONFIG_PATH} не найден — вставьте куки, файл будет создан")
            );
            return app;
        }

        match Config::load_async().await {
            Ok(config) => {
                app.config = config;
                app.sync_slice_inputs();

                if !app.config.has_cookie() {
                    app.screen = Screen::Cookies;
                    app.notice = Some(format!("{CONFIG_PATH} есть, но cookie пустые"));
                } else {
                    match app.rebuild_client() {
                        Ok(()) => {
                            app.screen = Screen::Title;
                        }
                        Err(error) => app.set_problem(error.to_string(), None, false),
                    }
                }
            }
            Err(error) => {
                app.screen = Screen::Cookies;
                app.notice = Some(error.to_string());
            }
        }

        app
    }

    pub async fn on_key(&mut self, key: KeyEvent) {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);

        if control && matches!(key.code, KeyCode::Char('c' | 'q')) {
            self.quit = true;
            return;
        }

        if self.switch_tab(&key) {
            return;
        }

        if self.problem.is_some() && matches!(self.screen, Screen::Chapters | Screen::Run) {
            match key.code {
                KeyCode::Char('r') => {
                    self.retry_problem().await;
                    return;
                }
                KeyCode::Char('c') => {
                    let retry = self.problem.as_ref().and_then(|problem| problem.retry);
                    self.open_cookies(retry);
                    return;
                }
                KeyCode::Esc => {
                    self.problem = None;
                    return;
                }
                _ => {}
            }
        }

        match self.screen {
            Screen::Cookies => self.on_key_cookies(key).await,
            Screen::Title => self.on_key_title(key).await,
            Screen::Chapters => self.on_key_chapters(key),
            Screen::Slice => self.on_key_slice(key).await,
            Screen::Confirm => self.on_key_confirm(key),
            Screen::Run => self.on_key_run(key),
        }
    }

    pub fn on_paste(&mut self, text: &str) {
        match self.screen {
            Screen::Cookies => {
                self.cookie.insert_str(text);
            }
            Screen::Title => {
                self.title_input.insert_str(text);
            }
            Screen::Slice => {
                self.slice_inputs[self.slice_focus].insert_str(text);
            }
            _ => {
                if self.search_active {
                    self.search.insert_str(text);
                }
            }
        }
    }

    pub async fn on_message(&mut self, message: Message) {
        match message {
            Message::ChaptersPage { title_id, cursor, chapters, next_cursor, total } => {
                self.problem = None;
                self.title_id = Some(title_id);
                self.chapter_total = total;

                if cursor == 0 {
                    self.chapters = chapters;
                } else {
                    self.chapters.extend(chapters);
                }

                if self.chapters.is_empty() {
                    self.notice = Some(format!("у произведения {title_id} не нашлось глав"));
                    self.busy = false;
                    return;
                }

                self.notice = None;
                self.busy = next_cursor.is_some();

                if let Some(cursor) = next_cursor {
                    let client = self.client.clone();
                    let messages = self.messages.clone();

                    if let Some(client) = client {
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                            tasks::spawn_chapters_page(client, title_id, cursor, messages);
                        });
                    }
                }
            }
            Message::Failed { retry, error, auth } => {
                self.busy = false;

                if
                    !self.run.running &&
                    matches!(retry, RetryAction::Chapters { .. }) &&
                    !self.chapters.is_empty()
                {
                    self.notice = Some(format!("не удалось догрузить список глав: {error}"));
                    return;
                }

                self.run.running = false;
                self.run.stage = "остановлено".to_owned();
                self.run.summary = Some("остановлено".to_owned());

                self.open_cookies(Some(retry));
                self.set_problem(error, Some(retry), auth);
            }
            Message::Progress(progress) => self.apply_progress(progress),
            Message::Finished(outcome) => {
                self.run.running = false;
                self.run.percent = 100;
                self.run.stage = "готово".to_owned();

                let pages = if outcome.pages_removed {
                    format!("страницы ({}) удалены", outcome.pages)
                } else {
                    format!("страницы оставлены: {}", outcome.pages_dir.display())
                };

                self.run.detail = format!(
                    "{} фрагментов · {pages} · {}",
                    outcome.slices,
                    outcome.slices_dir.display()
                );
                self.run.summary = Some(
                    format!("готово: {} страниц, {} фрагментов", outcome.pages, outcome.slices)
                );
                self.run.log.push(
                    format!("готово: {} страниц → {} фрагментов", outcome.pages, outcome.slices)
                );
            }
            Message::Client(ClientEvent::TokenRefreshed) => {
                self.run.log.push("cookie обновлены через refresh_token".to_owned());
            }
            Message::Client(ClientEvent::RefreshFailed(error)) => {
                self.notice = Some(format!("обновление cookie: {error}"));
            }
            Message::Client(ClientEvent::CookieWarning(warning)) => {
                self.notice = Some(format!("cookie-строка: {warning}"));
            }
        }
    }

    pub fn filtered(&self) -> Vec<usize> {
        let needle = self.search.value().trim().to_lowercase();

        self.chapters
            .iter()
            .enumerate()
            .filter(|(_, chapter)| {
                needle.is_empty() ||
                    chapter.title.to_lowercase().contains(&needle) ||
                    chapter.index.to_string().contains(&needle)
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn page_count(&self) -> usize {
        self.filtered().len().div_ceil(self.per_page()).max(1)
    }

    pub fn hotkeys(&self) -> Vec<(&'static str, &'static str)> {
        let mut keys: Vec<(&'static str, &'static str)> = vec![("←→", "вкладки (или 1-5)")];

        keys.extend(match self.screen {
            Screen::Cookies => vec![("Enter", "сохранить куки"), ("Esc", "назад")],
            Screen::Title => vec![("Enter", "загрузить список глав")],
            Screen::Chapters =>
                vec![
                    ("/", "поиск"),
                    ("↑↓", "выбор"),
                    ("PgUp/PgDn", "страница"),
                    ("Enter", "глава"),
                    ("R", "обновить"),
                    ("C", "куки"),
                    ("N", "тайтл")
                ],
            Screen::Slice =>
                vec![
                    ("Tab/↑↓", "поле"),
                    ("←→/Space", "переключатель"),
                    ("Enter", "запустить"),
                    ("Esc", "к списку")
                ],
            Screen::Confirm => vec![("Enter", "арендовать и скачать"), ("Esc", "назад")],
            Screen::Run => vec![("Esc", "прервать/назад"), ("C", "куки"), ("Q", "выход")],
        });

        if self.has_text_focus() {
            keys.push(("Ctrl+←→", "курсор в поле"));
        }

        keys
    }

    pub fn slice_fields(&self) -> usize {
        self.slice_inputs.len() + 1
    }

    pub fn per_page(&self) -> usize {
        self.list_height.get().max(1)
    }

    pub fn set_list_height(&self, height: usize) {
        self.list_height.set(height);
    }

    fn switch_tab(&mut self, key: &KeyEvent) -> bool {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Left if !control => {
                self.open_tab((self.screen.tab() as isize) - 1);
                true
            }
            KeyCode::Right if !control => {
                self.open_tab((self.screen.tab() as isize) + 1);
                true
            }
            KeyCode::Char(digit @ '1'..='5') if !control && !self.has_text_focus() => {
                self.open_tab((digit as isize) - ('1' as isize));
                true
            }
            _ => false,
        }
    }

    fn open_tab(&mut self, index: isize) {
        let screen = match index.rem_euclid(TAB_COUNT as isize) {
            0 => Screen::Cookies,
            1 => Screen::Title,
            2 => Screen::Chapters,
            3 => Screen::Slice,
            _ => Screen::Run,
        };

        self.screen = screen;
        self.search_active = false;
    }

    fn has_text_focus(&self) -> bool {
        match self.screen {
            Screen::Cookies | Screen::Title | Screen::Slice => true,
            Screen::Chapters => self.search_active,
            Screen::Confirm | Screen::Run => false,
        }
    }

    fn parse_field(&mut self, index: usize, name: &str) -> Option<usize> {
        match self.slice_inputs[index].value().trim().parse::<usize>() {
            Ok(value) => Some(value),
            Err(_) => {
                self.notice = Some(format!("{name} должен быть числом"));
                None
            }
        }
    }

    fn sync_slice_inputs(&mut self) {
        self.slice_inputs = [
            TextInput::new(self.config.spread.to_string()),
            TextInput::new(self.config.tolerance().to_string()),
            TextInput::new(self.config.step.to_string()),
            TextInput::new(self.config.out_dir.clone()),
        ];
    }

    pub fn tolerance_is_auto(&self) -> bool {
        self.config.tolerance.is_none()
    }

    fn rebuild_client(&mut self) -> Result<()> {
        let (events_tx, events_rx) = tasks::client_events();
        let client = KakaoClient::new(&self.config, Some(events_tx))?;
        tasks::spawn_client_forwarder(events_rx, self.messages.clone());
        self.client = Some(Arc::new(client));
        Ok(())
    }

    fn set_problem(&mut self, message: String, retry: Option<RetryAction>, auth: bool) {
        self.problem = Some(Problem {
            message: format!(
                "{message}\nЗапрос автоматически повторяется ({ATTEMPTS_LABEL}); если не помогло — введите куки заново."
            ),
            retry,
            auth,
        });
    }

    async fn retry_problem(&mut self) {
        let retry = self.problem.as_ref().and_then(|problem| problem.retry);
        match retry {
            Some(RetryAction::Chapters { title_id }) => {
                self.problem = None;
                self.title_id = Some(title_id);
                self.screen = Screen::Chapters;
                self.start_chapters_load();
            }
            Some(RetryAction::Run) => {
                self.problem = None;
                self.start_run();
            }
            None => {
                self.problem = None;
            }
        }
    }

    fn open_cookies(&mut self, retry: Option<RetryAction>) {
        self.pending_retry = retry;
        self.cookie.clear();
        self.previous_screen = self.screen;
        self.screen = Screen::Cookies;
    }

    fn start_chapters_load(&mut self) {
        if self.client.is_none() {
            self.notice = Some("клиент не готов: сначала введите куки".to_owned());
            self.screen = Screen::Cookies;
            return;
        }

        if self.title_id.is_none() {
            self.notice = Some("сначала введите ссылку на тайтл".to_owned());
            self.screen = Screen::Title;
            return;
        }

        self.busy = true;
        self.problem = None;
        self.chapters.clear();
        self.chapter_total = 0;
        self.selected = 0;
        self.page = 0;
        self.load_chapters_page(0);

        self.screen = Screen::Chapters;
    }

    fn load_chapters_page(&mut self, cursor: usize) {
        let Some(client) = self.client.clone() else {
            self.busy = false;
            self.notice = Some("клиент не готов: сначала введите куки".to_owned());
            return;
        };

        let Some(title_id) = self.title_id else {
            self.busy = false;
            return;
        };

        self.busy = true;
        tasks::spawn_chapters_page(client, title_id, cursor, self.messages.clone());
    }

    fn start_run(&mut self) {
        let (Some(client), Some(title_id), Some(chapter)) = (
            self.client.clone(),
            self.title_id,
            self.chapter.clone(),
        ) else {
            self.notice = Some("не выбран тайтл или глава".to_owned());
            self.screen = Screen::Chapters;
            return;
        };

        let options = SliceOptions {
            spread: self.config.spread,
            tolerance: self.config.tolerance(),
            step: self.config.step,
            out_dir: self.config.out_dir(),
        };

        let parallel = self.config.parallel;
        let mode = if parallel {
            format!("страницы параллельно, до {PARALLEL_PAGES} сразу")
        } else {
            "страницы по одной".to_owned()
        };

        self.run = RunState {
            stage: "подготовка".to_owned(),
            running: true,
            log: vec![format!("глава #{}, {mode}", chapter.index)],
            ..RunState::default()
        };
        self.screen = Screen::Run;
        self.problem = None;
        self.run_task = Some(
            tasks::spawn_run(
                client,
                title_id,
                chapter.index,
                options,
                parallel,
                self.messages.clone()
            )
        );
    }

    fn abort_run(&mut self) {
        if let Some(task) = self.run_task.take() {
            task.abort();
        }
        self.run.running = false;
        self.run.stage = "прервано".to_owned();
        self.run.summary = Some("прервано".to_owned());
    }

    fn apply_progress(&mut self, progress: Progress) {
        let (stage, percent, detail, done, total) = match progress {
            Progress::Chapter => ("глава", 3, "запрашиваю данные главы".to_owned(), 0, 0),
            Progress::Renting => ("аренда", 8, "арендую главу за билет".to_owned(), 0, 0),
            Progress::Pages => ("список страниц", 12, "запрашиваю список страниц".to_owned(), 0, 0),
            Progress::Page { done, total } =>
                (
                    "загрузка страниц",
                    12 + (((48 * done) / total.max(1)) as u16),
                    format!("{done}/{total} страниц"),
                    done,
                    total,
                ),
            Progress::EdgeMap { done, total } =>
                (
                    "анализ границ",
                    60 + (((20 * done) / total.max(1)) as u16),
                    format!("{done}/{total} изображений"),
                    done,
                    total,
                ),
            Progress::Slices { done, total } =>
                (
                    "нарезка",
                    80 + (((18 * done) / total.max(1)) as u16),
                    format!("{done}/{total} фрагментов"),
                    done,
                    total,
                ),
            Progress::Cleanup => ("очистка", 99, "удаляю страницы после нарезки".to_owned(), 0, 0),
        };

        let changed = self.run.stage != stage;

        if changed {
            self.run.log_done = 0;
        }

        let milestone = total == 0 || reached_milestone(done, total);

        if changed || (milestone && done > self.run.log_done) {
            self.run.log.push(format!("{stage}: {detail}"));
            self.run.log.truncate(200);
            self.run.log_done = done;
        }

        self.run.stage = stage.to_owned();
        self.run.percent = percent;
        self.run.detail = detail;
    }

    fn current_chapter(&self) -> Option<Chapter> {
        let filtered = self.filtered();
        let index = *filtered.get(self.selected)?;
        self.chapters.get(index).cloned()
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.filtered().len();
        if len == 0 {
            self.selected = 0;
            self.page = 0;
            return;
        }

        let position = ((self.selected as isize) + delta).clamp(0, (len - 1) as isize);
        self.selected = position as usize;
        self.page = self.selected / self.per_page();
    }

    fn change_page(&mut self, delta: isize) {
        let page_count = self.page_count() as isize;
        self.page = ((self.page as isize) + delta).clamp(0, page_count - 1) as usize;

        let start = self.page * self.per_page();
        if self.selected < start || self.selected >= start + self.per_page() {
            self.selected = start;
        }
    }

    async fn on_key_cookies(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let cookie = self.cookie.value().trim().to_owned();
                if cookie.is_empty() {
                    self.notice = Some("cookie-строка пустая".to_owned());
                    return;
                }

                self.config.cookie = cookie;

                if let Err(error) = self.config.save_async().await {
                    self.set_problem(error.to_string(), None, false);
                    return;
                }

                self.notice = Some(format!("{CONFIG_PATH} сохранён"));
                match self.rebuild_client() {
                    Ok(()) => {
                        self.problem = None;
                        self.screen = Screen::Title;

                        if self.pending_retry.take().is_some() {
                            self.retry_problem().await;
                        }
                    }
                    Err(error) => {
                        self.set_problem(error.to_string(), None, false);
                    }
                }
            }
            KeyCode::Esc => {
                if self.config.has_cookie() {
                    self.screen = self.previous_screen;
                } else {
                    self.quit = true;
                }
            }
            _ => {
                self.cookie.handle_key(key);
            }
        }
    }

    async fn on_key_title(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter =>
                match parse_title_id(self.title_input.value()) {
                    Ok(title_id) => {
                        self.title_id = Some(title_id);
                        self.search.clear();
                        self.notice = None;
                        self.screen = Screen::Chapters;
                        self.start_chapters_load();
                    }
                    Err(error) => {
                        self.notice = Some(error.to_string());
                    }
                }
            KeyCode::Esc => {
                self.quit = true;
            }
            _ => {
                self.title_input.handle_key(key);
            }
        }
    }

    fn on_key_chapters(&mut self, key: KeyEvent) {
        if self.search_active {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.search_active = false;
                }
                _ => {
                    if self.search.handle_key(key) {
                        self.selected = 0;
                        self.page = 0;
                    }
                }
            }
            return;
        }

        match key.code {
            KeyCode::Char('/') => {
                self.search_active = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
            }
            KeyCode::PageUp => {
                self.change_page(-1);
            }
            KeyCode::PageDown => {
                self.change_page(1);
            }
            KeyCode::Home => {
                self.selected = 0;
                self.page = 0;
            }
            KeyCode::End => {
                self.selected = self.filtered().len().saturating_sub(1);
                self.page = self.selected / self.per_page();
            }
            KeyCode::Enter => {
                if let Some(chapter) = self.current_chapter() {
                    self.chapter = Some(chapter);
                    self.screen = Screen::Slice;
                    self.notice = None;
                }
            }
            KeyCode::Char('r') => {
                self.start_chapters_load();
            }
            KeyCode::Char('c') => {
                self.open_cookies(None);
            }
            KeyCode::Char('n') => {
                self.screen = Screen::Title;
            }
            KeyCode::Esc => {
                self.quit = true;
            }
            _ => {}
        }
    }

    async fn on_key_slice(&mut self, key: KeyEvent) {
        let fields = self.slice_fields();

        match key.code {
            KeyCode::Tab | KeyCode::Down => {
                self.slice_focus = (self.slice_focus + 1) % fields;
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.slice_focus = (self.slice_focus + fields - 1) % fields;
            }
            KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right if
                self.slice_focus == SLICE_TOGGLE
            => {
                self.toggle_parallel().await;
            }
            KeyCode::Enter => {
                self.apply_slice_params().await;
            }
            KeyCode::Esc => {
                self.screen = Screen::Chapters;
            }
            _ => {
                self.slice_inputs[self.slice_focus].handle_key(key);
            }
        }
    }

    async fn toggle_parallel(&mut self) {
        self.config.parallel = !self.config.parallel;

        if let Err(error) = self.config.save_async().await {
            self.set_problem(error.to_string(), None, false);
            return;
        }

        self.notice = Some(
            if self.config.parallel {
                format!("параллельная загрузка включена: до {PARALLEL_PAGES} страниц одновременно")
            } else {
                "параллельная загрузка выключена: страницы качаются по одной".to_owned()
            }
        );
    }

    async fn apply_slice_params(&mut self) {
        let spread = match self.parse_field(0, "spread") {
            Some(value) => value,
            None => {
                return;
            }
        };
        let tolerance = match self.parse_field(1, "tolerance") {
            Some(value) => value,
            None => {
                return;
            }
        };
        let step = match self.parse_field(2, "step") {
            Some(value) => value,
            None => {
                return;
            }
        };
        let out_dir = self.slice_inputs[3].value().trim().to_owned();

        self.config.spread = spread;
        self.config.tolerance = if tolerance == auto_tolerance(spread) {
            None
        } else {
            Some(tolerance)
        };
        self.config.step = step;
        self.config.out_dir = out_dir;

        if let Err(error) = self.config.validate() {
            self.notice = Some(error.to_string());
            return;
        }

        if let Err(error) = self.config.save_async().await {
            self.set_problem(error.to_string(), None, false);
            return;
        }

        self.notice = Some(format!("параметры сохранены в {CONFIG_PATH}"));

        if !self.chapter.as_ref().is_some_and(Chapter::needed_rental) {
            self.start_run();
        } else {
            self.screen = Screen::Confirm;
        }
    }

    fn on_key_confirm(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.start_run(),
            KeyCode::Esc => {
                self.screen = Screen::Slice;
            }
            _ => {}
        }
    }

    fn on_key_run(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if self.run.running {
                    self.abort_run();
                } else {
                    self.screen = Screen::Chapters;
                }
            }
            KeyCode::Char('c') => {
                self.open_cookies(None);
            }
            KeyCode::Char('q') => {
                self.quit = true;
            }
            _ => {}
        }
    }
}

fn reached_milestone(done: usize, total: usize) -> bool {
    let step = (total / 10).max(1);
    done == total || done.is_multiple_of(step)
}
