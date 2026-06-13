fn pad(s: &str, width: usize) -> String {
    let s_len = s.chars().count();
    if s_len >= width {
        s.to_string()
    } else {
        format!("{} {}", s, ".".repeat(width - s_len - 1))
    }
}

fn convert_currency(val: &str) -> String {
    if val.is_empty() || val == "-" || val == "—" || val == "0" {
        return val.to_string();
    }

    let clean_val = val
        .replace(',', ".")
        .chars()
        .filter(|c| c.is_numeric() || *c == '.')
        .collect::<String>();
    if let Ok(num) = clean_val.parse::<f64>() {
        let (converted, cur) = if val.contains('₹') || val.to_lowercase().contains("inr") {
            (num * 0.012, "USD")
        } else if val.contains('₽')
            || val.to_lowercase().contains("rub")
            || val.to_lowercase().contains("pуб")
        {
            (num * 0.011, "USD")
        } else if val.contains('₴') || val.to_lowercase().contains("uah") {
            (num * 0.025, "USD")
        } else if val.contains("CHF") {
            (num * 1.12, "USD")
        } else if val.contains('£') {
            (num * 1.27, "USD")
        } else if val.contains("ARS") {
            (num * 0.0012, "USD")
        } else if val.contains("TRY") || val.contains("TL") {
            (num * 0.031, "USD")
        } else if val.contains("€") {
            (num * 1.08, "USD")
        } else {
            return val.to_string();
        };

        if converted > 0.0 {
            return format!("{} (~{:.2} {})", val, converted, cur);
        }
    }

    val.to_string()
}

use crate::cli::app::{App, AppMode};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap,
    },
};

const TEXT_NORM: Color = Color::Rgb(210, 200, 230);

const TEXT_DIM: Color = Color::Rgb(110, 95, 135);

const ACCENT_1: Color = Color::Rgb(160, 130, 220);

const ACCENT_2: Color = Color::Rgb(130, 170, 220);

const OK_COLOR: Color = Color::Rgb(130, 200, 150);

const WARN_COLOR: Color = Color::Rgb(200, 180, 120);

const ERR_COLOR: Color = Color::Rgb(200, 120, 130);

const BG_COLOR: Color = Color::Rgb(45, 35, 55);

pub fn draw_ui(f: &mut Frame, app: &mut App) {
    let area = f.area();

    f.render_widget(Block::default().style(Style::default().bg(BG_COLOR)), area);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(f, app, root[0]);

    if app.active_tab == crate::cli::app::AppTab::Network {
        draw_network_tab(f, app, root[1]);
    } else {
        draw_body(f, app, root[1]);
    }

    draw_footer(f, app, root[2]);

    match app.mode {
        AppMode::SelectCookiesMethod => draw_select_popup(f, app, area, " Load Cookies "),
        AppMode::SelectProxiesMethod => draw_select_popup(f, app, area, " Load Proxies "),
        AppMode::InputPath => draw_text_input_popup(f, app, area, " Enter Cookies File Path "),
        AppMode::InputProxyPath => draw_text_input_popup(f, app, area, " Enter Proxies File Path "),
        AppMode::PasteText => draw_textarea_popup(
            f,
            app,
            area,
            " Paste Cookies | Double Enter to submit / ESC to cancel ",
        ),
        AppMode::PasteProxyText => draw_textarea_popup(
            f,
            app,
            area,
            " Paste Proxies | Double Enter to submit / ESC to cancel ",
        ),

        _ => {}
    }
}

fn draw_header(f: &mut Frame, _app: &mut App, area: Rect) {
    let subtitle = "-- account extractor & validator -- | Made by xbl1e".to_string();

    let text = vec![
        Line::from(vec![Span::styled(
            "█▀ ▀█▀ █▀▀ ▄▀█ █▀▄▀█   █▀▀ █▀█ █▀█ █▄▀ █ █▀▀   █▀ █▀▀ ▄▀█ █▀█ █▀▀ █░█ █▀▀ █▀█",
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "▄█ ░█░ ██▄ █▀█ █░▀░█   █▄▄ █▄█ █▄█ █░█ █ ██▄   ▄█ ██▄ █▀█ █▀▄ █▄▄ █▀█ ██▄ █▀▄",
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(subtitle, Style::default().fg(TEXT_DIM))]),
    ];

    let p = Paragraph::new(text).alignment(Alignment::Center);

    f.render_widget(p, area);
}

fn draw_body(f: &mut Frame, app: &mut App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(28), Constraint::Percentage(70)])
        .split(area);

    let left_col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(cols[0]);

    draw_accounts(f, app, left_col[0]);

    draw_logs(f, app, left_col[1]);

    let right_col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(cols[1]);

    draw_details(f, app, right_col[0]);

    draw_proxies(f, app, right_col[1]);
}

fn draw_proxies(f: &mut Frame, app: &mut App, area: Rect) {
    let title = format!(" proxies ({}) ", app.proxies.len());

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_DIM));

    if app.proxies.is_empty() {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  no proxies loaded (optional)",
                Style::default().fg(TEXT_DIM),
            )),
        ])
        .block(block);

        f.render_widget(p, area);

        return;
    }

    let header_cells = ["Proto", "Address", "Port", "Loc"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(TEXT_DIM).add_modifier(Modifier::BOLD)));

    let header = Row::new(header_cells)
        .style(Style::default())
        .height(1)
        .bottom_margin(1);

    let rows = app.proxies.iter().map(|p| {
        let url = &p.url;

        let (proto, rest) = url.split_once("://").unwrap_or(("", url));

        let (ip, port) = rest.rsplit_once(':').unwrap_or((rest, ""));

        let proto_cell = Cell::from(proto).style(Style::default().fg(ACCENT_1));

        let ip_cell = Cell::from(ip).style(Style::default().fg(TEXT_NORM));

        let port_cell = Cell::from(port).style(Style::default().fg(ACCENT_2));

        let loc_str = if p.country_code.is_empty() || p.country_code == "UNKNOWN" {
            "--".to_string()
        } else {
            p.country_code.clone()
        };

        let loc_cell = Cell::from(loc_str).style(Style::default().fg(OK_COLOR));

        Row::new(vec![proto_cell, ip_cell, port_cell, loc_cell])
    });

    let t = Table::new(
        rows,
        [
            Constraint::Length(7),
            Constraint::Min(15),
            Constraint::Length(7),
            Constraint::Length(4),
        ],
    )
    .header(header)
    .block(block)
    .column_spacing(2);

    f.render_widget(t, area);
}

fn draw_accounts(f: &mut Frame, app: &mut App, area: Rect) {
    let title = format!(" accounts ({}) ", app.accounts.len());

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_DIM));

    if app.accounts.is_empty() {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  no accounts yet",
                Style::default().fg(TEXT_DIM),
            )),
        ])
        .block(block);

        f.render_widget(p, area);

        return;
    }

    let items: Vec<ListItem> = app
        .accounts
        .iter()
        .enumerate()
        .map(|(i, acc)| {
            let is_sel = i == app.selected_account;

            let mut name = acc.display("username");

            if name == "—" || name.is_empty() {
                name = &acc.steam_id;
            }

            let vac_ban =
                acc.vac == "⛔" || acc.vac == "✗" || acc.vac.to_lowercase().contains("ban");

            let is_prime = acc.cs_prime == "✔"
                || acc.cs_prime == "✓"
                || acc.cs_prime.to_lowercase().contains("yes")
                || acc.cs_prime.contains("Prime");

            let is_limited = acc.limited.contains("Limited");

            let mut spans = vec![Span::styled(" ", Style::default())];

            let mut name_style = Style::default();

            if is_sel {
                name_style = name_style
                    .fg(Color::Black)
                    .bg(ACCENT_1)
                    .add_modifier(Modifier::BOLD);
            } else {
                name_style = name_style.fg(TEXT_DIM);
            }

            if !acc.is_valid {
                name_style = name_style.fg(ERR_COLOR).remove_modifier(Modifier::BOLD);
                spans.push(Span::styled("[EXPIRED] ", Style::default().fg(ERR_COLOR)));
            } else {
                spans.push(Span::styled(
                    "[V] ",
                    Style::default().fg(OK_COLOR).add_modifier(Modifier::BOLD),
                ));
            }

            if vac_ban {
                spans.push(Span::styled("[BAN] ", Style::default().fg(ERR_COLOR)));
            } else {
                if is_prime {
                    spans.push(Span::styled("[Prime] ", Style::default().fg(OK_COLOR)));
                }
                if is_limited {
                    spans.push(Span::styled("[L] ", Style::default().fg(WARN_COLOR)));
                }
            }

            spans.push(Span::styled(name.to_string(), name_style));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(ACCENT_1).fg(Color::Black));

    f.render_stateful_widget(list, area, &mut app.account_state);
}

fn draw_logs(f: &mut Frame, app: &mut App, area: Rect) {
    let title = " logs ".to_string();

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_DIM));

    if app.logs.is_empty() {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  no logs available.",
                Style::default().fg(TEXT_DIM),
            )),
        ])
        .block(block);

        f.render_widget(p, area);

        return;
    }

    let items: Vec<ListItem> = app
        .logs
        .iter()
        .map(|msg| {
            let (prefix_span, text_span) = if msg.starts_with("error:") || msg.starts_with("[!]") {
                let p = msg.chars().take(6).collect::<String>();
                let t = msg.chars().skip(6).collect::<String>();
                (
                    Span::styled(
                        p,
                        Style::default()
                            .fg(Color::LightRed)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(t, Style::default().fg(TEXT_DIM)),
                )
            } else if msg.starts_with("[+]") || msg.starts_with("+ ") {
                let p = msg.chars().take(6).collect::<String>();
                let t = msg.chars().skip(6).collect::<String>();
                (
                    Span::styled(
                        p,
                        Style::default()
                            .fg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(t, Style::default().fg(TEXT_DIM)),
                )
            } else if msg.starts_with("[-] ") || msg.starts_with("- ") {
                let p = msg.chars().take(6).collect::<String>();
                let t = msg.chars().skip(6).collect::<String>();
                (
                    Span::styled(
                        p,
                        Style::default()
                            .fg(Color::LightRed)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(t, Style::default().fg(TEXT_DIM)),
                )
            } else if msg.starts_with("> ") {
                let p = msg.chars().take(6).collect::<String>();
                let t = msg.chars().skip(6).collect::<String>();
                (
                    Span::styled(
                        p,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(t, Style::default().fg(TEXT_DIM)),
                )
            } else {
                (
                    Span::raw(""),
                    Span::styled(msg.clone(), Style::default().fg(TEXT_DIM)),
                )
            };

            ListItem::new(Line::from(vec![prefix_span, text_span]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(list, area, &mut app.logs_state);
}

fn draw_footer(f: &mut Frame, app: &mut App, area: Rect) {
    let other_tab = if app.active_tab == crate::cli::app::AppTab::Network {
        "Main"
    } else {
        "Network"
    };

    let keys = get_key_hints(app, other_tab);

    let mut spans = vec![];
    for (k, v) in keys {
        spans.push(Span::styled(
            format!(" [{}] ", k),
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!("{}  ", v),
            Style::default().fg(TEXT_NORM),
        ));
    }

    let footer = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(footer, area);
}

fn get_key_hints(app: &App, other_tab: &str) -> Vec<(&'static str, String)> {
    if app.active_tab == crate::cli::app::AppTab::Network {
        vec![
            ("Tab", format!("Switch to {}", other_tab)),
            ("\u{2191}/\u{2193}", "Select Request".to_string()),
            ("\u{2190}/\u{2192}", "Switch Info Tab".to_string()),
            ("PgUp/PgDn", "Scroll Content".to_string()),
            ("q", "Quit".to_string()),
        ]
    } else {
        let mut v = vec![
            ("Tab", format!("Switch to {}", other_tab)),
            ("\u{2191}\u{2193}", "Select".to_string()),
        ];
        if !app.accounts.is_empty() {
            v.push(("o", "Browser".to_string()));
        }
        v.push(("c", "Cookies".to_string()));
        v.push(("p", "Proxies".to_string()));
        v.push(("i", "Import".to_string()));
        if !app.accounts.is_empty() {
            v.push(("e", "Export".to_string()));
            v.push(("d", "Delete".to_string()));
        }
        v.push(("q", "Quit".to_string()));
        v
    }
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(popup_layout[1])[1]
}

fn draw_select_popup(f: &mut Frame, _app: &mut App, area: Rect, title: &str) {
    let popup = centered_rect(30, 4, area);

    f.render_widget(Clear, popup);
    f.render_widget(Block::default().style(Style::default().bg(BG_COLOR)), popup);

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(ACCENT_1)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_1))
        .style(Style::default().bg(BG_COLOR));

    let text = vec![
        Line::from(vec![
            Span::styled(
                "  [1] ",
                Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
            ),
            Span::styled("From file path", Style::default().fg(TEXT_NORM)),
        ]),
        Line::from(vec![
            Span::styled(
                "  [2] ",
                Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Paste Text", Style::default().fg(TEXT_NORM)),
        ]),
    ];

    let p = Paragraph::new(text).block(block);

    f.render_widget(p, popup);
}

fn draw_text_input_popup(f: &mut Frame, app: &mut App, area: Rect, title: &str) {
    let popup = centered_rect(60, 3, area);

    f.render_widget(Clear, popup);
    f.render_widget(Block::default().style(Style::default().bg(BG_COLOR)), popup);

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(ACCENT_1)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_1))
        .style(Style::default().bg(BG_COLOR));

    let p = Paragraph::new(app.input_buffer.as_str()).block(block);

    f.render_widget(p, popup);
}

fn draw_textarea_popup(f: &mut Frame, app: &mut App, area: Rect, title: &str) {
    let popup = centered_rect(60, 12, area);

    f.render_widget(Clear, popup);
    f.render_widget(Block::default().style(Style::default().bg(BG_COLOR)), popup);

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(ACCENT_1)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_1))
        .style(Style::default().bg(BG_COLOR));

    let p = Paragraph::new(app.input_buffer.as_str())
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(p, popup);
}

fn colorize_val(val: &str) -> Span<'static> {
    let clean_val = strip_ansi_codes(val);

    if clean_val.is_empty() || clean_val == "-" || clean_val == "—" {
        return Span::styled("—", Style::default().fg(TEXT_DIM));
    }

    let color = determine_value_color(&clean_val);
    Span::styled(clean_val, Style::default().fg(color))
}

fn strip_ansi_codes(val: &str) -> String {
    let mut clean_val = val.to_string();
    if clean_val.contains("[32m") || clean_val.contains("[0m") || clean_val.contains("[31m") {
        clean_val = clean_val
            .replace("\x1b[32m", "")
            .replace("\x1b[0m", "")
            .replace("\x1b[31m", "")
            .replace("[32m", "")
            .replace("[0m", "")
            .replace("[31m", "");
    }
    clean_val
}

fn determine_value_color(clean_val: &str) -> Color {
    let lower = clean_val.to_lowercase();
    if ["clean", "unrestricted", "linked", "prime", "active"].contains(&lower.as_str()) {
        OK_COLOR
    } else if lower.contains("disabled")
        || lower.contains("banned")
        || lower.contains("limited")
        || lower.contains("unlinked")
    {
        ERR_COLOR
    } else if lower.contains("private") || lower.contains("empty") || lower.contains("non-prime") {
        TEXT_DIM
    } else if clean_val.contains("CHF")
        || clean_val.contains("$")
        || clean_val.contains("€")
        || clean_val.contains("₹")
        || clean_val.contains("₽")
        || clean_val.contains("£")
    {
        OK_COLOR
    } else {
        TEXT_NORM
    }
}

fn draw_details(f: &mut Frame, app: &mut App, area: Rect) {
    if app.accounts.is_empty() {
        let block = Block::default()
            .title(Span::styled(
                " Account Info ",
                Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(TEXT_DIM));
        let msg = if app.mode == AppMode::Scanning {
            "  scanning..."
        } else {
            "  press [c] to load cookies - [p] to load proxies"
        };
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(msg, Style::default().fg(TEXT_DIM))),
        ])
        .block(block);
        f.render_widget(p, area);
        return;
    }

    let acc = &app.accounts[app.selected_account];

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(vertical_chunks[0]);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vertical_chunks[1]);

    let info_block = Block::default()
        .title(Span::styled(
            " Account Info ",
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_DIM));

    let info_rows = vec![
        Row::new(vec![
            Cell::from(pad("Username", 14)),
            Cell::from(colorize_val(&acc.username)),
        ]),
        Row::new(vec![
            Cell::from(pad("Custom URL", 14)),
            Cell::from(colorize_val(&acc.custom_url)),
        ]),
        Row::new(vec![
            Cell::from(pad("Steam ID", 14)),
            Cell::from(colorize_val(&acc.steam_id)),
        ]),
        Row::new(vec![
            Cell::from(pad("Level", 14)),
            Cell::from(colorize_val(&acc.level)),
        ]),
        Row::new(vec![
            Cell::from(pad("Created", 14)),
            Cell::from(colorize_val(&acc.member_since)),
        ]),
        Row::new(vec![
            Cell::from(pad("Email", 14)),
            Cell::from(colorize_val(&acc.email)),
        ]),
        Row::new(vec![
            Cell::from(pad("Phone", 14)),
            Cell::from(colorize_val(&acc.phone)),
        ]),
        Row::new(vec![
            Cell::from(pad("Country", 14)),
            Cell::from(colorize_val(&acc.country)),
        ]),
        Row::new(vec![
            Cell::from(pad("Family View", 14)),
            Cell::from(colorize_val(&acc.family_view)),
        ]),
    ];
    let info_table = Table::new(
        info_rows,
        [Constraint::Percentage(30), Constraint::Percentage(70)],
    )
    .block(info_block);
    f.render_widget(info_table, top_chunks[0]);

    let sec_block = Block::default()
        .title(Span::styled(
            " Security & Status ",
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_DIM));

    let sec_rows = vec![
        Row::new(vec![
            Cell::from(pad("Guard", 14)),
            Cell::from(colorize_val(&acc.guard)),
        ]),
        Row::new(vec![
            Cell::from(pad("VAC Ban", 14)),
            Cell::from(colorize_val(&acc.vac)),
        ]),
        Row::new(vec![
            Cell::from(pad("Trade Ban", 14)),
            Cell::from(colorize_val(&acc.trade_ban)),
        ]),
        Row::new(vec![
            Cell::from(pad("Comm. Ban", 14)),
            Cell::from(colorize_val(&acc.community_ban)),
        ]),
        Row::new(vec![
            Cell::from(pad("Account Type", 14)),
            Cell::from(colorize_val(&acc.limited)),
        ]),
        Row::new(vec![
            Cell::from(pad("Market", 14)),
            Cell::from(colorize_val(&acc.market)),
        ]),
        Row::new(vec![
            Cell::from(pad("CS2 Prime", 14)),
            Cell::from(colorize_val(&acc.cs_prime)),
        ]),
        Row::new(vec![
            Cell::from(pad("SIH Link", 14)),
            Cell::from(colorize_val(&acc.sih_status)),
        ]),
    ];
    let sec_table = Table::new(
        sec_rows,
        [Constraint::Percentage(45), Constraint::Percentage(55)],
    )
    .block(sec_block);
    f.render_widget(sec_table, top_chunks[1]);

    let assets_block = Block::default()
        .title(Span::styled(
            " Assets & Stats ",
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_DIM));

    let assets_rows = vec![
        {
            let converted = convert_currency(&acc.wallet_balance);
            Row::new(vec![
                Cell::from(pad("Wallet", 14)),
                Cell::from(colorize_val(&converted)),
            ])
        },
        {
            let hold = convert_currency(&acc.hold_balance);
            Row::new(vec![
                Cell::from(pad("Hold Balance", 14)),
                Cell::from(colorize_val(&hold)),
            ])
        },
        {
            let inv = convert_currency(&acc.inventory_balance);
            Row::new(vec![
                Cell::from(pad("Inv. Balance", 14)),
                Cell::from(colorize_val(&inv)),
            ])
        },
        Row::new(vec![
            Cell::from(pad("Steam Points", 14)),
            Cell::from(colorize_val(&acc.steam_points)),
        ]),
        Row::new(vec![
            Cell::from(pad("Games Owned", 14)),
            Cell::from(colorize_val(&acc.games_count)),
        ]),
        Row::new(vec![
            Cell::from(pad("Hours Played", 14)),
            Cell::from(colorize_val(&acc.hours_played)),
        ]),
        Row::new(vec![
            Cell::from(pad("Friends", 14)),
            Cell::from(colorize_val(&acc.friends_count)),
        ]),
        Row::new(vec![
            Cell::from(pad("Wishlist", 14)),
            Cell::from(colorize_val(&acc.wishlist_count)),
        ]),
        Row::new(vec![
            Cell::from(pad("Active Sales", 14)),
            Cell::from(colorize_val(&acc.market_active_listings)),
        ]),
        Row::new(vec![
            Cell::from(pad("Badges", 14)),
            Cell::from(colorize_val(&acc.badges)),
        ]),
    ];
    let assets_table = Table::new(
        assets_rows,
        [Constraint::Percentage(40), Constraint::Percentage(60)],
    )
    .block(assets_block);
    f.render_widget(assets_table, bottom_chunks[0]);

    let inv_block = Block::default()
        .title(Span::styled(
            " Game Inventories ",
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_DIM));

    let inv_rows = vec![
        Row::new(vec![
            Cell::from(pad("CS2", 14)),
            Cell::from(colorize_val(&acc.inventory_cs2)),
        ]),
        Row::new(vec![
            Cell::from(pad("Dota 2", 14)),
            Cell::from(colorize_val(&acc.inventory_dota)),
        ]),
        Row::new(vec![
            Cell::from(pad("TF2", 14)),
            Cell::from(colorize_val(&acc.inventory_tf2)),
        ]),
        Row::new(vec![
            Cell::from(pad("PUBG", 14)),
            Cell::from(colorize_val(&acc.inventory_pubg)),
        ]),
        Row::new(vec![
            Cell::from(pad("Rust", 14)),
            Cell::from(colorize_val(&acc.inventory_rust)),
        ]),
        Row::new(vec![
            Cell::from(pad("Steam", 14)),
            Cell::from(colorize_val(&acc.inventory_steam)),
        ]),
    ];
    let inv_table = Table::new(
        inv_rows,
        [Constraint::Percentage(30), Constraint::Percentage(70)],
    )
    .block(inv_block);
    f.render_widget(inv_table, bottom_chunks[1]);
}

fn highlight_json(json_str: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(json_str);

    let text = match parsed {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| json_str.to_string()),
        Err(_) => json_str.to_string(),
    };

    for (i, line) in text.lines().take(100).enumerate() {
        let mut spans = vec![Span::styled(
            format!("{:>3} │ ", i + 1),
            Style::default().fg(TEXT_DIM),
        )];
        if text.trim().starts_with('<') {
            let mut in_tag = false;
            let mut current_span = String::new();

            for c in line.chars() {
                if c == '<' {
                    if !current_span.is_empty() {
                        spans.push(Span::styled(
                            current_span.clone(),
                            Style::default().fg(TEXT_NORM),
                        ));
                        current_span.clear();
                    }
                    in_tag = true;
                    current_span.push(c);
                } else if c == '>' && in_tag {
                    current_span.push(c);
                    spans.push(Span::styled(
                        current_span.clone(),
                        Style::default().fg(ACCENT_1),
                    ));
                    current_span.clear();
                    in_tag = false;
                } else {
                    current_span.push(c);
                }
            }
            if !current_span.is_empty() {
                if in_tag {
                    spans.push(Span::styled(current_span, Style::default().fg(ACCENT_1)));
                } else {
                    spans.push(Span::styled(current_span, Style::default().fg(TEXT_NORM)));
                }
            }
        } else if let Some(idx) = line.find("\": ") {
            let key_part = &line[..idx + 3];
            let rest = &line[idx + 3..];
            spans.push(Span::styled(
                key_part.to_string(),
                Style::default().fg(ACCENT_1),
            ));
            if rest.starts_with('"') {
                spans.push(Span::styled(
                    rest.to_string(),
                    Style::default().fg(OK_COLOR),
                ));
            } else if rest.starts_with("true")
                || rest.starts_with("false")
                || rest.starts_with("null")
            {
                spans.push(Span::styled(
                    rest.to_string(),
                    Style::default().fg(ERR_COLOR),
                ));
            } else if rest
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() || c == '-')
            {
                spans.push(Span::styled(
                    rest.to_string(),
                    Style::default().fg(WARN_COLOR),
                ));
            } else {
                spans.push(Span::styled(
                    rest.to_string(),
                    Style::default().fg(TEXT_NORM),
                ));
            }
        } else {
            spans.push(Span::styled(
                line.to_string(),
                Style::default().fg(TEXT_NORM),
            ));
        }
        lines.push(Line::from(spans));
    }

    if text.lines().count() > 100 {
        lines.push(Line::from(Span::styled(
            "... (truncated)",
            Style::default().fg(TEXT_DIM),
        )));
    }

    lines
}

fn draw_network_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let list_block = Block::default()
        .title(Span::styled(
            " Requests ",
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_DIM));

    let selected_idx = app.network_state.selected().unwrap_or(0);

    let items: Vec<ListItem> = app
        .network_requests
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let is_sel = i == selected_idx;
            let method_color = match r.method.as_str() {
                "GET" => OK_COLOR,
                "POST" => WARN_COLOR,
                "PUT" | "PATCH" => Color::Rgb(120, 160, 210),
                "DELETE" => ERR_COLOR,
                _ => TEXT_DIM,
            };

            let mut spans = vec![];
            if is_sel {
                spans.push(Span::styled(
                    format!("{:4} ", r.method),
                    Style::default()
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ));
                if let Some(idx) = r.url.find("://") {
                    let (proto, rest) = r.url.split_at(idx + 3);
                    spans.push(Span::styled(proto, Style::default().fg(Color::DarkGray)));
                    spans.push(Span::styled(rest, Style::default().fg(Color::Black)));
                } else {
                    spans.push(Span::styled(
                        r.url.clone(),
                        Style::default().fg(Color::Black),
                    ));
                }
            } else {
                spans.push(Span::styled(
                    format!("{:4} ", r.method),
                    Style::default()
                        .fg(method_color)
                        .add_modifier(Modifier::BOLD),
                ));
                if let Some(idx) = r.url.find("://") {
                    let (proto, rest) = r.url.split_at(idx + 3);
                    spans.push(Span::styled(proto, Style::default().fg(Color::DarkGray)));
                    spans.push(Span::styled(rest, Style::default().fg(Color::Gray)));
                } else {
                    spans.push(Span::styled(
                        r.url.clone(),
                        Style::default().fg(Color::Gray),
                    ));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(list_block)
        .highlight_style(Style::default().bg(ACCENT_1).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, chunks[0], &mut app.network_state);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    let details_block = Block::default()
        .title(Span::styled(
            " Details ",
            Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_DIM));

    if let Some(req) = app.network_requests.get(selected_idx) {
        let status = req.status.unwrap_or(0);
        let status_color = if (200..300).contains(&status) {
            OK_COLOR
        } else {
            ERR_COLOR
        };
        let status_line = vec![
            Span::styled(
                format!(" {} ", status),
                Style::default()
                    .bg(status_color)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}ms ", req.duration_ms),
                Style::default().fg(TEXT_DIM),
            ),
            Span::styled(req.url.clone(), Style::default().fg(TEXT_NORM)),
        ];

        let mut tab_headers = vec![Span::raw("  ")];
        let tabs = [
            ("Headers", crate::cli::app::InnerNetworkTab::Headers),
            ("Body", crate::cli::app::InnerNetworkTab::Body),
            ("Cookies", crate::cli::app::InnerNetworkTab::Cookies),
        ];
        for (name, tab) in tabs.iter() {
            if std::mem::discriminant(&app.network_inner_tab) == std::mem::discriminant(tab) {
                tab_headers.push(Span::styled(
                    format!(" {} ", name),
                    Style::default()
                        .bg(Color::Rgb(120, 100, 150))
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                tab_headers.push(Span::styled(
                    format!(" {} ", name),
                    Style::default().fg(TEXT_DIM),
                ));
            }
            tab_headers.push(Span::raw(" "));
        }

        let inner_area = details_block.inner(right_chunks[0]);
        f.render_widget(details_block, right_chunks[0]);

        let detail_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(inner_area);

        let header_lines = vec![
            Line::from(status_line),
            Line::from(tab_headers),
            Line::from(""),
        ];
        f.render_widget(Paragraph::new(header_lines), detail_chunks[0]);

        let mut lines = vec![];

        match app.network_inner_tab {
            crate::cli::app::InnerNetworkTab::Headers => {
                if req.req_headers.is_empty() && req.res_headers.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "No headers recorded",
                        Style::default().fg(TEXT_DIM),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        " ▼ Request Headers ",
                        Style::default()
                            .bg(ACCENT_1)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )));
                    for (k, v) in &req.req_headers {
                        lines.push(Line::from(vec![
                            Span::styled(format!("{}: ", k), Style::default().fg(ACCENT_2)),
                            Span::raw(v),
                        ]));
                    }
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        " ▼ Response Headers ",
                        Style::default()
                            .bg(ACCENT_1)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )));
                    for (k, v) in &req.res_headers {
                        lines.push(Line::from(vec![
                            Span::styled(format!("{}: ", k), Style::default().fg(ACCENT_2)),
                            Span::raw(v),
                        ]));
                    }
                }
            }
            crate::cli::app::InnerNetworkTab::Body => {
                let body_str = req.response_body.as_deref().unwrap_or("");
                lines.extend(highlight_json(body_str));
            }
            crate::cli::app::InnerNetworkTab::Cookies => {
                let mut found_cookies = false;
                lines.push(Line::from(Span::styled(
                    " ▼ Request Cookies ",
                    Style::default()
                        .bg(ACCENT_1)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )));
                for (k, v) in &req.req_headers {
                    if k.eq_ignore_ascii_case("cookie") {
                        found_cookies = true;
                        lines.push(Line::from(vec![
                            Span::styled("Cookie: ", Style::default().fg(ACCENT_2)),
                            Span::raw(v),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    " ▼ Response Cookies (Set-Cookie) ",
                    Style::default()
                        .bg(ACCENT_1)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )));
                for (k, v) in &req.res_headers {
                    if k.eq_ignore_ascii_case("set-cookie") {
                        found_cookies = true;
                        lines.push(Line::from(vec![
                            Span::styled("Set-Cookie: ", Style::default().fg(ACCENT_2)),
                            Span::raw(v),
                        ]));
                    }
                }
                if !found_cookies {
                    lines.push(Line::from(Span::styled(
                        "No cookies recorded in this request",
                        Style::default().fg(TEXT_DIM),
                    )));
                }
            }
        }
        let p = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((app.network_scroll as u16, 0));
        f.render_widget(p, detail_chunks[1]);

        let stats_block = Block::default()
            .title(Span::styled(
                " Stats ",
                Style::default().fg(ACCENT_1).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(TEXT_DIM));
        let stats_area = stats_block.inner(right_chunks[1]);
        f.render_widget(stats_block, right_chunks[1]);

        let total = req.duration_ms.max(1);
        let connect_ms = req.duration_ms / 4;
        let request_ms = req.duration_ms / 6;
        let response_ms = req.duration_ms.saturating_sub(connect_ms + request_ms);

        let w = 40.0;
        let c_w = ((connect_ms as f64 / total as f64) * w).round() as usize;
        let req_w = ((request_ms as f64 / total as f64) * w).round() as usize;
        let res_w = ((response_ms as f64 / total as f64) * w).round() as usize;

        let c_bar = "█".repeat(c_w.max(1));
        let req_bar = "█".repeat(req_w.max(1));
        let res_bar = "█".repeat(res_w.max(1));
        let space_req = " ".repeat(c_w);
        let space_res = " ".repeat(c_w + req_w);

        let c_text = format!("v Connect  ({:>4} ms)", connect_ms);
        let req_text = format!("v Request  ({:>4} ms)", request_ms);
        let res_text = format!("v Response ({:>4} ms)", response_ms);

        let mut slines = vec![];
        slines.push(Line::from(vec![
            Span::styled(format!("{:20} ", c_text), Style::default().fg(TEXT_DIM)),
            Span::styled(&c_bar, Style::default().fg(Color::DarkGray)),
        ]));
        slines.push(Line::from(vec![
            Span::styled(format!("{:20} ", req_text), Style::default().fg(TEXT_DIM)),
            Span::raw(space_req),
            Span::styled(&req_bar, Style::default().fg(Color::Rgb(255, 165, 0))),
        ]));
        slines.push(Line::from(vec![
            Span::styled(format!("{:20} ", res_text), Style::default().fg(TEXT_DIM)),
            Span::raw(space_res),
            Span::styled(&res_bar, Style::default().fg(Color::Rgb(100, 150, 255))),
        ]));

        slines.push(Line::from(""));

        let res_size = req.response_body.as_ref().map(|b| b.len()).unwrap_or(0);
        let req_size = req
            .req_headers
            .iter()
            .map(|(k, v)| k.len() + v.len() + 4)
            .sum::<usize>()
            + req.url.len();

        slines.push(Line::from(Span::styled(
            format!("Request Size .... {} B", req_size),
            Style::default().fg(TEXT_DIM),
        )));

        if res_size > 0 {
            let kb = res_size as f64 / 1024.0;
            slines.push(Line::from(Span::styled(
                format!("Response Size ... {:.2} KB", kb),
                Style::default().fg(TEXT_DIM),
            )));

            if total > 0 {
                let speed_kbs = (res_size as f64 / 1024.0) / (total as f64 / 1000.0);
                slines.push(Line::from(Span::styled(
                    format!("Transfer Rate ... {:.2} KB/s", speed_kbs),
                    Style::default().fg(TEXT_DIM),
                )));
            }
        } else {
            slines.push(Line::from(Span::styled(
                "Response Size ... 0 KB",
                Style::default().fg(TEXT_DIM),
            )));
        }

        let content_type = req
            .res_headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == "content-type")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        slines.push(Line::from(Span::styled(
            format!("Content-Type .... {}", content_type),
            Style::default().fg(TEXT_DIM),
        )));

        f.render_widget(Paragraph::new(slines), stats_area);
    } else {
        f.render_widget(
            Paragraph::new("No request selected").block(details_block),
            right_chunks[0],
        );
    }
}
