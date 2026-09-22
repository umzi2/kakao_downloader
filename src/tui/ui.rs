use ratatui::{
    Frame,
    layout::{ Constraint, Layout, Rect },
    style::{ Color, Modifier, Style },
    text::{ Line, Span },
    widgets::{ Block, BorderType, Gauge, List, Paragraph, Tabs, Wrap },
};
use unicode_width::{ UnicodeWidthChar, UnicodeWidthStr };

use crate::{
    config::auto_tolerance,
    download::PARALLEL_PAGES,
    kakao::chapters::chapter::{ Chapter, Purchase },
    tui::app::{ App, SLICE_TOGGLE, Screen },
};

const ACCENT: Color = Color::Cyan;
const OK: Color = Color::Green;
const WARN: Color = Color::Yellow;
const BAD: Color = Color::Red;
const DIM: Color = Color::DarkGray;

const TABS: [&str; 5] = ["1 Куки", "2 Тайтл", "3 Главы", "4 Слайс", "5 Загрузка"];

const BADGE_WIDTH: usize = 14;

const CONTENT_WIDTH: usize = 96;

const LIST_PADDING: usize = 2;

const INDEX_WIDTH: usize = 6;

const LABEL_WIDTH: usize = 28;

const COOKIE_HELP: [&str; 6] = [
    "1. Откройте https://page.kakao.com/ и войдите в аккаунт.",
    "2. F12 → вкладка Network → обновите страницу (F5).",
    "3. Выберите запрос к page.kakao.com и найдите заголовок Cookie.",
    "4. Скопируйте значение целиком (Copy value) и вставьте сюда.",
    "5. Enter — сохранить. Куки обновляются, пока жив refresh_token.",
    "Esc без сохранённых куки — выход.",
];

pub fn render(frame: &mut Frame, app: &App) {
    let status_height = if app.problem.is_some() { 5 } else { 2 };
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(status_height),
    ]);

    let chunks = layout.split(frame.area());

    render_tabs(frame, app, chunks[0]);

    match app.screen {
        Screen::Cookies => render_cookies(frame, app, chunks[1]),
        Screen::Title => render_title(frame, app, chunks[1]),
        Screen::Chapters => render_chapters(frame, app, chunks[1]),
        Screen::Slice => render_slice(frame, app, chunks[1]),
        Screen::Confirm => render_confirm(frame, app, chunks[1]),
        Screen::Run => render_run(frame, app, chunks[1]),
    }

    render_status(frame, app, chunks[2]);
}

fn panel(title: impl Into<String>) -> Block<'static> {
    Block::bordered().border_type(BorderType::Rounded).title(title.into())
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = Tabs::new(TABS.to_vec())
        .select(app.screen.tab())
        .padding(" ", " ")
        .divider(Span::styled("│", Style::new().fg(DIM)))
        .block(panel(" Kakao Downloader "))
        .highlight_style(
            Style::new().fg(Color::Rgb(18, 22, 30)).bg(ACCENT).add_modifier(Modifier::BOLD)
        );

    frame.render_widget(tabs, area);
}

fn render_cookies(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length((COOKIE_HELP.len() as u16) + 2),
        Constraint::Min(1),
    ]);
    let chunks = layout.split(area);

    let field = Paragraph::new(app.cookie.visible(true))
        .style(Style::new().fg(OK))
        .block(panel(" Cookie (заголовок целиком) "));
    frame.render_widget(field, chunks[0]);

    let help: Vec<Line> = COOKIE_HELP.iter()
        .map(|line| Line::styled(*line, Style::new().fg(DIM)))
        .collect();
    frame.render_widget(
        Paragraph::new(help).wrap(Wrap { trim: false }).block(panel(" Как получить ")),
        chunks[1]
    );

    let headline = match &app.problem {
        Some(problem) if problem.auth =>
            Some(
                Line::styled(
                    "Куки недействительны: войдите на page.kakao.com и вставьте заголовок Cookie заново.",
                    Style::new().fg(BAD).add_modifier(Modifier::BOLD)
                )
            ),
        _ => None,
    };

    let notice = app.notice
        .as_ref()
        .map(|notice| Line::styled(notice.clone(), Style::new().fg(WARN)));

    if headline.is_some() || notice.is_some() {
        let lines: Vec<Line> = headline.into_iter().chain(notice).collect();
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), chunks[2]);
    }
}

fn render_title(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(1),
    ]);
    let chunks = layout.split(area);

    let field = Paragraph::new(app.title_input.visible(true))
        .style(Style::new().fg(OK))
        .block(panel(" Ссылка на тайтл "));
    frame.render_widget(field, chunks[0]);

    frame.render_widget(
        Paragraph::new(
            vec![
                Line::styled(
                    "например https://page.kakao.com/content/54801072/ или просто 54801072",
                    Style::new().fg(DIM)
                ),
                Line::raw("")
            ]
        ).wrap(Wrap { trim: false }),
        chunks[1]
    );

    if let Some(notice) = &app.notice {
        frame.render_widget(
            Paragraph::new(Line::styled(notice.clone(), Style::new().fg(WARN))).wrap(Wrap {
                trim: true,
            }),
            chunks[2]
        );
    } else if app.busy {
        frame.render_widget(
            Paragraph::new(Line::styled("загружаю список глав…", Style::new().fg(ACCENT))),
            chunks[2]
        );
    }
}

fn render_chapters(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]);
    let chunks = layout.split(area);

    let search_style = if app.search_active {
        Style::new().fg(OK).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(DIM)
    };

    let search = if app.search.value().trim().is_empty() && !app.search_active {
        Line::from(
            vec![
                Span::styled("поиск ", search_style),
                Span::styled("/  название или номер главы", search_style)
            ]
        )
    } else {
        Line::from(
            vec![
                Span::styled("поиск ", search_style),
                Span::raw(app.search.visible(app.search_active))
            ]
        )
    };
    frame.render_widget(Paragraph::new(search), chunks[0]);

    let inner_height = chunks[1].height.saturating_sub(2) as usize;
    app.set_list_height(inner_height);

    let per_page = app.per_page();
    let filtered = app.filtered();
    let start = app.page * per_page;

    let inner_width = chunks[1].width.saturating_sub(2) as usize;
    let lead = LIST_PADDING.min(inner_width.saturating_sub(24));
    let content = (inner_width - lead).min(CONTENT_WIDTH);
    let title_width = content.saturating_sub(INDEX_WIDTH + 2 + BADGE_WIDTH).max(12);

    let rows: Vec<Line> = filtered
        .iter()
        .skip(start)
        .take(per_page)
        .enumerate()
        .filter_map(|(offset, index)| app.chapters.get(*index).map(|chapter| (offset, chapter)))
        .map(|(offset, chapter)| {
            let selected = start + offset == app.selected;
            chapter_line(chapter, selected, title_width, lead)
        })
        .collect();

    let stats = if app.busy {
        format!(
            " тайтл {} · загружено {} из {} · загрузка… ",
            app.title_id.map(|id| id.to_string()).unwrap_or_else(|| "—".to_owned()),
            app.chapters.len(),
            app.chapter_total
        )
    } else {
        format!(
            " тайтл {} · глав {} · стр {}/{} ",
            app.title_id.map(|id| id.to_string()).unwrap_or_else(|| "—".to_owned()),
            app.chapters.len(),
            (app.page + 1).min(app.page_count()),
            app.page_count()
        )
    };

    let block = panel(" Главы ")
        .title_top(Line::styled(stats, Style::new().fg(ACCENT)).right_aligned())
        .title_bottom(legend());

    frame.render_widget(List::new(rows).block(block), chunks[1]);
}

fn legend() -> Line<'static> {
    Line::from(
        vec![
            Span::styled(" ✓ арендована ", Style::new().fg(OK)),
            Span::styled("·", Style::new().fg(DIM)),
            Span::styled(" ◆ за билет ", Style::new().fg(ACCENT)),
            Span::styled("·", Style::new().fg(DIM)),
            Span::styled(" ✕ нельзя ", Style::new().fg(BAD)),
            Span::styled("·", Style::new().fg(DIM)),
            Span::styled(" ? куки устарели ", Style::new().fg(WARN))
        ]
    )
}

fn chapter_line(
    chapter: &Chapter,
    selected: bool,
    title_width: usize,
    lead: usize
) -> Line<'static> {
    let (badge, color) = badge(chapter);

    let line = Line::from(
        vec![
            Span::raw(" ".repeat(lead)),
            Span::styled(
                format!("{:>width$} ", format!("#{}", chapter.index), width = INDEX_WIDTH - 1),
                Style::new().fg(if selected { ACCENT } else { DIM })
            ),
            Span::raw(pad_right(&truncate_width(&chapter.title, title_width), title_width)),
            Span::raw("  "),
            Span::styled(pad_left(badge, BADGE_WIDTH), Style::new().fg(color))
        ]
    );

    if selected {
        line.style(Style::new().bg(Color::Rgb(32, 62, 92)).add_modifier(Modifier::BOLD))
    } else {
        line
    }
}

fn badge(chapter: &Chapter) -> (&'static str, Color) {
    match (chapter.purchase, chapter.rentable) {
        (Purchase::Rent, _) => ("✓ арендована", OK),
        (Purchase::Unknown, _) => ("? куки устарели", WARN),
        (Purchase::NotPurchased, true) => ("◆ за билет", ACCENT),
        (Purchase::NotPurchased, false) => ("✕ нельзя", BAD),
    }
}

fn render_slice(frame: &mut Frame, app: &App, area: Rect) {
    let labels = ["высота фрагмента:", "разброс высоты:", "шаг сканирования:", "каталог вывода:"];

    let params_height = (labels.len() as u16) + 3;
    let layout = Layout::vertical([
        Constraint::Length(params_height),
        Constraint::Length(8),
        Constraint::Min(1),
    ]);
    let chunks = layout.split(area);

    let mut lines = Vec::new();
    for (index, label) in labels.iter().enumerate() {
        let focused = index == app.slice_focus;
        let style = if focused {
            Style::new().fg(OK).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(DIM)
        };

        lines.push(
            Line::from(
                vec![
                    Span::styled(format!("{label:<LABEL_WIDTH$}"), style),
                    Span::raw(app.slice_inputs[index].visible(focused))
                ]
            )
        );
    }

    let focused = app.slice_focus == SLICE_TOGGLE;
    let label_style = if focused {
        Style::new().fg(OK).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(DIM)
    };
    let (value, value_style) = if app.config.parallel {
        ("‹ да ›", Style::new().fg(OK).add_modifier(Modifier::BOLD))
    } else {
        ("‹ нет ›", Style::new().fg(DIM))
    };

    lines.push(
        Line::from(
            vec![
                Span::styled(format!("{:<LABEL_WIDTH$}", "параллельные страницы"), label_style),
                Span::styled(value, value_style),
                Span::styled(format!("   до {PARALLEL_PAGES} страниц сразу"), Style::new().fg(DIM))
            ]
        )
    );

    frame.render_widget(Paragraph::new(lines).block(panel(" Параметры нарезки ")), chunks[0]);

    let chapter = app.chapter
        .as_ref()
        .map(|chapter| format!("#{} {}", chapter.index, truncate_width(&chapter.title, 60)))
        .unwrap_or_else(|| "не выбрана".to_owned());

    let mut lines = vec![
        Line::from(
            vec![
                Span::styled("глава       ", Style::new().fg(DIM)),
                Span::styled(chapter, Style::new().fg(ACCENT))
            ]
        ),
        Line::from(
            vec![Span::styled("страницы    ", Style::new().fg(DIM)), if app.config.parallel {
                Span::styled(
                    format!("параллельно, до {PARALLEL_PAGES} сразу"),
                    Style::new().fg(ACCENT)
                )
            } else {
                Span::styled("по одной", Style::new().fg(ACCENT))
            }]
        ),
        Line::from(
            vec![
                Span::styled("разброс     ", Style::new().fg(DIM)),
                Span::raw(
                    if app.tolerance_is_auto() {
                        format!("авто, spread / 8 = {}", auto_tolerance(app.config.spread))
                    } else {
                        format!("{} вручную", app.config.tolerance())
                    }
                )
            ]
        ),
        Line::from(
            vec![
                Span::styled("результат   ", Style::new().fg(DIM)),
                Span::raw("<out dir>/<глава>/slices/, страницы удаляются после нарезки")
            ]
        ),
        Line::from(
            vec![
                Span::styled("config      ", Style::new().fg(DIM)),
                Span::raw("параметры сохраняются автоматически")
            ]
        )
    ];

    if app.config.parallel {
        lines.push(
            Line::styled(
                "внимание: параллельные запросы сервер может принять за бота",
                Style::new().fg(WARN)
            )
        );
    } else {
        lines.push(Line::raw(""));
    }

    if let Some(notice) = &app.notice {
        lines.push(Line::styled(notice.clone(), Style::new().fg(WARN)));
    }

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(panel(" Загрузка ")),
        chunks[1]
    );
}

fn render_confirm(frame: &mut Frame, app: &App, area: Rect) {
    let chapter = app.chapter
        .as_ref()
        .map(|chapter| format!("#{} {}", chapter.index, chapter.title))
        .unwrap_or_else(|| "глава не выбрана".to_owned());

    let lines = vec![
        Line::styled(
            "Глава не арендована, но её можно арендовать за билет",
            Style::new().fg(WARN).add_modifier(Modifier::BOLD)
        ),
        Line::raw(""),
        Line::raw(chapter),
        Line::raw(""),
        Line::raw("Enter — списать билет, арендовать главу и начать загрузку."),
        Line::styled("Esc — вернуться к параметрам нарезки, не тратя билет.", Style::new().fg(DIM))
    ];

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(panel(" Подтверждение аренды ")),
        area
    );
}

fn render_run(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(4),
    ]);
    let chunks = layout.split(area);

    let chapter = app.chapter
        .as_ref()
        .map(|chapter| format!("#{} {}", chapter.index, truncate_width(&chapter.title, 60)))
        .unwrap_or_else(|| "глава не выбрана".to_owned());

    frame.render_widget(
        Paragraph::new(
            Line::from(
                vec![
                    Span::styled("глава ", Style::new().fg(DIM)),
                    Span::styled(chapter, Style::new().add_modifier(Modifier::BOLD))
                ]
            )
        ),
        chunks[0]
    );

    let gauge = Gauge::default()
        .block(panel(" Прогресс "))
        .ratio((app.run.percent as f64) / 100.0)
        .gauge_style(Style::new().fg(ACCENT).bg(Color::Rgb(30, 36, 46)))
        .label(format!("{}%", app.run.percent));
    frame.render_widget(gauge, chunks[1]);

    frame.render_widget(
        Paragraph::new(
            Line::from(
                vec![
                    Span::styled(
                        format!("{} ", app.run.stage),
                        Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
                    ),
                    Span::styled(app.run.detail.clone(), Style::new().fg(WARN))
                ]
            )
        ),
        chunks[2]
    );

    let height = chunks[3].height.saturating_sub(2) as usize;
    let log: Vec<Line> = app.run.log
        .iter()
        .rev()
        .take(height)
        .rev()
        .map(|line| Line::styled(line.clone(), Style::new().fg(DIM)))
        .collect();

    let title = match &app.run.summary {
        Some(summary) => format!(" Журнал · {summary} "),
        None => " Журнал ".to_owned(),
    };

    frame.render_widget(Paragraph::new(log).block(panel(title)), chunks[3]);
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let lines = match &app.problem {
        Some(problem) => {
            let mut lines: Vec<Line> = problem.message
                .lines()
                .enumerate()
                .map(|(index, text)| {
                    let prefix = if index == 0 { "Проблема: " } else { "" };
                    Line::styled(format!("{prefix}{text}"), Style::new().fg(BAD))
                })
                .collect();

            lines.push(
                chips(
                    &[
                        ("R", "повторить"),
                        ("C", "ввести куки заново"),
                        ("Esc", "скрыть"),
                    ]
                )
            );

            lines
        }
        None =>
            match &app.notice {
                Some(notice) =>
                    vec![
                        Line::styled(notice.clone(), Style::new().fg(WARN)),
                        chips(&app.hotkeys())
                    ],
                None => vec![chips(&app.hotkeys())],
            }
    };

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
}

fn chips(keys: &[(&'static str, &'static str)]) -> Line<'static> {
    let mut spans = Vec::new();

    for (index, (key, description)) in keys.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  │  ", Style::new().fg(DIM)));
        }

        spans.push(
            Span::styled((*key).to_owned(), Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))
        );
        spans.push(Span::styled(format!(" {description}"), Style::new().fg(DIM)));
    }

    Line::from(spans)
}

fn truncate_width(text: &str, limit: usize) -> String {
    if UnicodeWidthStr::width(text) <= limit {
        return text.to_owned();
    }

    let mut shortened = String::new();
    let mut width = 0usize;

    for character in text.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if width + character_width > limit.saturating_sub(1) {
            break;
        }

        shortened.push(character);
        width += character_width;
    }

    shortened.push('…');
    shortened
}

fn pad_right(text: &str, width: usize) -> String {
    let current = UnicodeWidthStr::width(text);
    let mut padded = text.to_owned();
    padded.push_str(&" ".repeat(width.saturating_sub(current)));
    padded
}

fn pad_left(text: &str, width: usize) -> String {
    let current = UnicodeWidthStr::width(text);
    let mut padded = " ".repeat(width.saturating_sub(current));
    padded.push_str(text);
    padded
}
