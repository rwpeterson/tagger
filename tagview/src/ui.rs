use tagtools::TSTEP;

use crate::app::{App, SettingsMode, Grain};

#[allow(unused_imports)]
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::{
        Axis, BarChart, Block, Borders, Cell, Chart, Clear, Dataset, Gauge, LineGauge, List,
        ListItem, ListState, Paragraph, Row, Sparkline, Table, Tabs, Wrap,
    },
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.size());
    draw_titlebar(f, app, chunks[0]);
    draw_tabbar(f, app, chunks[1]);
    match app.tabs.index {
        0 => draw_counts_tab(f, app, chunks[2]),
        1 => draw_settings_tab(f, app, chunks[2]),
        _ => {}
    }
    draw_footer(f, app, chunks[3]);
}

fn draw_settings_tab<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)].as_ref())
        .split(area);
    let mut help = None;
    if app.live_settings {
        let state = app.settings_state.as_mut().unwrap();

        help = Some(Paragraph::new(vec![Spans::from(
            vec![
                Span::raw("  wasd "),
                Span::styled("navigate settings", Style::default().add_modifier(Modifier::DIM)),
                Span::raw("  e/q "),
                Span::styled("enter/leave setting", Style::default().add_modifier(Modifier::DIM)),
                Span::raw(" w/s "),
                Span::styled("change setting", Style::default().add_modifier(Modifier::DIM)),
                Span::raw(" r/f "),
                Span::styled("change step size [", Style::default().add_modifier(Modifier::DIM)),
                match state.grain {
                    Grain::Fine => Span::styled("▓", Style::default().fg(Color::LightCyan)),
                    Grain::Medium => Span::styled("▓▓", Style::default().fg(Color::LightCyan)),
                    Grain::Coarse => Span::styled("▓▓▓", Style::default().fg(Color::LightCyan)),
                },
                match state.grain {
                    Grain::Fine => Span::raw("░░"),
                    Grain::Medium => Span::raw("░"),
                    Grain::Coarse => Span::raw(""),
                },
                Span::styled("]", Style::default().add_modifier(Modifier::DIM)),
            ]
        )]));
    }
    f.render_widget(help.unwrap_or_else(|| Paragraph::new(Span::raw(""))), chunks[0]);
    draw_settings_tab_body(f, app, chunks[1]);
}

fn draw_settings_tab_body<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    if app.live_settings == false {
        let text = vec![
            Spans::from("User input required:"),
            Spans::from(""),
            Spans::from("x - Get current channel settings from tagger"),
            Spans::from("    (Gets only channels with a singles subscription in your config)"),
            Spans::from(""),
            Spans::from("m - Set tagger to use channel settings specified in your config"),
            Spans::from("    (Sets all channel_settings in config)"),
        ];
        f.render_widget(Paragraph::new(text), area);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(14),
                    Constraint::Length(13),
                    Constraint::Length(9),
                    Constraint::Length(3),
                    Constraint::Min(1),
                ]
                .as_ref(),
            )
            .split(area);
        let state = app.settings_state.as_mut().unwrap();
        let channel_items: Vec<ListItem> = state
            .channel_settings
            .iter()
            .map(|rs| ListItem::new(format!("Channel {: >2}", rs.ch)))
            .collect();
        let channel_list = List::new(channel_items)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">");
        f.render_stateful_widget(channel_list, chunks[0], &mut state.ch_state);
        let delay_items: Vec<ListItem> = state
            .channel_settings
            .iter()
            .map(|rs| {
                let mut s = numfmt(rs.del as f64 * TSTEP, 2);
                s.push('s');
                ListItem::new(s)
            })
            .collect();
        let delay_list = List::new(delay_items)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray))
            .highlight_style(match state.mode {
                SettingsMode::Delay(None) => {
                    Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
                }
                SettingsMode::Delay(Some(_)) => Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::REVERSED | Modifier::BOLD),
                _ => Style::default().add_modifier(Modifier::BOLD),
            });
        f.render_stateful_widget(delay_list, chunks[1], &mut state.ch_state);
        let threshold_items: Vec<ListItem> = state
            .channel_settings
            .iter()
            .map(|rs| {
                let mut s = numfmt(rs.thr, 3);
                s.push('V');
                ListItem::new(s)
            })
            .collect();
        let threshold_list = List::new(threshold_items)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray))
            .highlight_style(match state.mode {
                SettingsMode::Threshold(None) => {
                    Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
                }
                SettingsMode::Threshold(Some(_)) => Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::REVERSED | Modifier::BOLD),
                _ => Style::default().add_modifier(Modifier::BOLD),
            });
        f.render_stateful_widget(threshold_list, chunks[2], &mut state.ch_state);
        let inversion_items: Vec<ListItem> = state
            .channel_settings
            .iter()
            .map(|rs| {
                let s = match rs.inv {
                    true => String::from("-"),
                    false => String::from("+"),
                };
                ListItem::new(s)
            })
            .collect();
        let inversion_list = List::new(inversion_items)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray))
            .highlight_style(match state.mode {
                SettingsMode::Invert(None) => {
                    Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
                }
                SettingsMode::Invert(Some(_)) => Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::REVERSED | Modifier::BOLD),
                _ => Style::default().add_modifier(Modifier::BOLD),
            });
        f.render_stateful_widget(inversion_list, chunks[3], &mut state.ch_state);

        let pats = app.pats.lock();
        let mut coincvec = pats
            .iter()
            .filter(|(m, _)| m.count_ones() == 2)
            .collect::<Vec<_>>();
        coincvec.sort();
        let coinc_items: Vec<ListItem> = coincvec
            .iter()
            .map(|(&m, &ct)| {
                let mut bi = bit_iter::BitIter::from(m);
                let ch_b = bi.next().unwrap() + 1;
                let ch_a = bi.next().unwrap() + 1;
                let s = format!("{0}-{1}: {2}", ch_b, ch_a, ct);
                ListItem::new(s)
            })
            .collect();
        let mut x = ListState::default();
        let coinc_list = List::new(coinc_items)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_stateful_widget(coinc_list, chunks[4], &mut x)
    }
}

fn draw_counts_tab<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(1)].as_ref())
        .split(area);
    draw_singles(f, app, chunks[0]);
    draw_coincidences(f, app, chunks[1]);
}

fn draw_titlebar<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let text = vec![Spans::from(vec![
        Span::styled(
            if app.save {
                " Time Tagger RECORDING "
            } else {
                " Time Tagger "
            },
            Style::default()
                .fg(if app.save { Color::Red } else { Color::Green })
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        ),
        Span::raw(" Keys:  Ctrl+C "),
        Span::styled("quit", Style::default().add_modifier(Modifier::DIM)),
        Span::raw("  c "),
        Span::styled(
            "clear error flags",
            Style::default().add_modifier(Modifier::DIM),
        ),
        Span::raw("  Ctrl+R "),
        Span::styled("record tags", Style::default().add_modifier(Modifier::DIM)),
        Span::raw("  ←/→ "),
        Span::styled("cycle tabs", Style::default().add_modifier(Modifier::DIM)),
    ])];
    f.render_widget(Paragraph::new(text), area);
}

fn draw_tabbar<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let titles = app
        .tabs
        .titles
        .iter()
        //.map(|&t| Spans::from(Span::styled(t, Style::default().fg(Color::Gray))))
        .map(|&t| Spans::from(Span::raw(t)))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD))
        .select(app.tabs.index);
    f.render_widget(tabs, area);
}

fn _draw_sparkline<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let data = app
        .coincs
        .iter()
        .map(|h| *h.get(&(1, 2)).unwrap_or(&0.0))
        .collect::<Vec<f64>>();
    let udata = data.iter().map(|&r| r as u64).collect::<Vec<u64>>();
    let sparkline = Sparkline::default()
        .block(Block::default().title(format!(
            "Coincidences (1, 2): {:>7.0}",
            data.last().unwrap_or(&0.0)
        )))
        .style(Style::default().fg(Color::Red))
        .data(&udata);
    f.render_widget(sparkline, area);
}

fn draw_singles<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let pats = app.pats.lock();
    let nch = pats.len();
    let ncols = 8;
    let nrows = match nch % ncols {
        0 => nch / ncols,
        _ => nch / ncols + 1,
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage((100.0 / nrows as f32) as u16);
            nrows
        ])
        .split(area);

    let mut rc: Vec<Vec<Rect>> = Vec::new();
    for row in rows {
        rc.push(
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Percentage((100.0 / ncols as f32) as u16);
                    ncols
                ])
                .split(row),
        );
    }

    let dur = app.duration;
    let mut singlesvec = pats
        .iter()
        .filter(|(m, _)| m.count_ones() == 1)
        .collect::<Vec<_>>();
    singlesvec.sort();
    let mut chan_iter = singlesvec.iter();
    for row in rc {
        for elem in row {
            if let Some((&m, &ct)) = chan_iter.next() {
                let ch = bit_iter::BitIter::from(m).next().unwrap() + 1;
                let rate = ct as f64 / (dur as f64 * 5e-9);
                let text = Paragraph::new(Spans::from(vec![
                    Span::styled(
                        format!("{:>2}", ch),
                        Style::default().add_modifier(Modifier::BOLD | Modifier::DIM),
                    ),
                    Span::styled(format!("{:>7.0}", rate), Style::default()),
                ]));
                f.render_widget(text, elem);
            }
        }
    }
}

fn draw_coincidences<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let pats = app.pats.lock();
    let nch = pats.len();
    let ncols = 8;
    let nrows = match nch % ncols {
        0 => nch / ncols,
        _ => nch / ncols + 1,
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage((100.0 / nrows as f32) as u16);
            nrows
        ])
        .split(area);

    let mut rc: Vec<Vec<Rect>> = Vec::new();
    for row in rows {
        rc.push(
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Percentage((100.0 / ncols as f32) as u16);
                    ncols
                ])
                .split(row),
        );
    }

    let dur = app.duration;
    let mut coincvec = pats
        .iter()
        .filter(|(m, _)| m.count_ones() == 2)
        .collect::<Vec<_>>();
    coincvec.sort();
    let mut chan_iter = coincvec.iter();
    for row in rc {
        for elem in row {
            if let Some((&m, &ct)) = chan_iter.next() {
                let mut bi = bit_iter::BitIter::from(m);
                let ch_b = bi.next().unwrap() + 1;
                let ch_a = bi.next().unwrap() + 1;
                let rate = ct as f64 / (dur as f64 * 5e-9);
                let text = Paragraph::new(Spans::from(vec![
                    Span::styled(
                        format!("{0}-{1}", ch_b, ch_a),
                        Style::default().add_modifier(Modifier::BOLD | Modifier::DIM),
                    ),
                    Span::styled(numfmt(rate, 0), Style::default()),
                ]));
                f.render_widget(text, elem);
            }
        }
    }
}

fn draw_footer<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let mut errtxt: Vec<Span> = match app.flags.is_empty() {
        true => {
            vec![Span::styled("None", Style::default().fg(Color::Green))]
        }
        false => app
            .flags
            .iter()
            .map(|f| {
                vec![
                    Span::styled(
                        f,
                        Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::REVERSED),
                    ),
                    Span::raw(" "),
                ]
            })
            .flatten()
            .collect(),
    };
    let mut text = vec![
        Span::styled(
            format!(
                "Working dir: {}",
                std::env::current_dir().unwrap().to_str().unwrap()
            ),
            Style::default(),
        ),
        Span::raw(" "),
        Span::styled("Errors: ", Style::default()),
        Span::raw(""),
    ];
    text.append(&mut errtxt);

    f.render_widget(Paragraph::new(Spans::from(text)), area);
}

/// Human-readable string encoding size in bytes with largest metric prefix to three decimal places
pub fn sizefmt(bytes: usize) -> String {
    let oom = (bytes as f64).log10().floor() as u32;
    let pfx = oom as u32 / 3 * 3;
    let rpfx = pfx.saturating_sub(3);
    let value = bytes / 10usize.pow(rpfx);
    let unit = match pfx {
        0 => Some("B"),
        3 => Some("kB"),
        6 => Some("MB"),
        9 => Some("GB"),
        12 => Some("TB"),
        15 => Some("PB"),
        _ => None,
    };
    let int_part = value / 10usize.pow(3);
    let frac_part = value % 10usize.pow(3);
    let space = match unit {
        Some(u) => format!("{0:>3}.{1:03} {2}", int_part, frac_part, u),
        None => format!("{} B", bytes),
    };
    return space;
}

pub fn numfmt(num: f64, dec: usize) -> String {
    let mut buffer = ryu::Buffer::new();
    let raw = buffer.format(num);
    if raw.contains("e") {
        // num < 1e-5 || num >= 1e16
        let eidx = raw.find('e').unwrap();
        let (mantissa, exponent) = raw.split_at(eidx);
        let mut mantissa = mantissa.to_string();
        mantissa.truncate(dec + 2);
        return format!("{}{}", mantissa, exponent);
    } else {
        let pidx = raw.find('.').unwrap();
        let mut truncated = raw.to_string();
        let d = if dec > 0 { dec + 1 } else { 0 };
        truncated.truncate(pidx + d);
        return truncated;
    }
}
