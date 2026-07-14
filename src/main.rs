mod game;

use game::{Direction, Difficulty, GameState, Position, SnakeGame};
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Widget},
    Frame, Terminal,
};
use std::{
    error::Error,
    io::{self, stdout},
    time::{Duration, Instant},
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
        original_hook(panic_info);
    }));
}

fn main() -> Result<(), Box<dyn Error>> {
    setup_panic_hook();
    
    enable_raw_mode()?;
    let mut stdout_handle = stdout();
    execute!(stdout_handle, EnterAlternateScreen, crossterm::cursor::Hide)?;
    
    let backend = CrosstermBackend::new(stdout_handle);
    let mut terminal = Terminal::new(backend)?;
    
    let mut game = SnakeGame::new();
    
    let res = run_app(&mut terminal, &mut game);
    
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::cursor::Show)?;
    
    if let Err(err) = res {
        println!("Error running game: {:?}", err);
    }
    
    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    game: &mut SnakeGame,
) -> Result<(), Box<dyn Error>>
where
    B::Error: Error + 'static,
{
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| draw_ui(f, game))?;
        
        let tick_rate = match game.state {
            GameState::Playing => game.tick_rate(),
            _ => Duration::from_millis(50),
        };
        
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));
            
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    if handle_key_input(game, key) {
                        return Ok(());
                    }
                }
            }
        }
        
        if game.state == GameState::Playing && last_tick.elapsed() >= tick_rate {
            game.tick();
            last_tick = Instant::now();
        }
    }
}

fn handle_key_input(game: &mut SnakeGame, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }
    
    match game.state {
        GameState::StartScreen => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('w') => {
                game.menu_selection = (game.menu_selection + 3) % 4;
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('s') => {
                game.menu_selection = (game.menu_selection + 1) % 4;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                game.difficulty = match game.menu_selection {
                    0 => Difficulty::Easy,
                    1 => Difficulty::Medium,
                    2 => Difficulty::Hard,
                    _ => Difficulty::Insane,
                };
                game.reset();
                game.state = GameState::Playing;
            }
            _ => {}
        },
        GameState::Playing => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                game.state = GameState::StartScreen;
            }
            KeyCode::Char(' ') | KeyCode::Char('p') => {
                game.state = GameState::Paused;
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('w') => {
                if game.current_dir != Direction::Down {
                    game.next_dir = Direction::Up;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('s') => {
                if game.current_dir != Direction::Up {
                    game.next_dir = Direction::Down;
                }
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('a') => {
                if game.current_dir != Direction::Right {
                    game.next_dir = Direction::Left;
                }
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('d') => {
                if game.current_dir != Direction::Left {
                    game.next_dir = Direction::Right;
                }
            }
            KeyCode::Char('r') => {
                game.reset();
            }
            _ => {}
        },
        GameState::Paused => match key.code {
            KeyCode::Char(' ') | KeyCode::Char('p') => {
                game.state = GameState::Playing;
            }
            KeyCode::Char('r') => {
                game.reset();
                game.state = GameState::Playing;
            }
            KeyCode::Char('m') | KeyCode::Esc => {
                game.state = GameState::StartScreen;
            }
            KeyCode::Char('q') => {
                return true;
            }
            _ => {}
        },
        GameState::GameOver => match key.code {
            KeyCode::Char('r') => {
                game.reset();
                game.state = GameState::Playing;
            }
            KeyCode::Char('m') | KeyCode::Esc => {
                game.state = GameState::StartScreen;
            }
            KeyCode::Char('q') => {
                return true;
            }
            _ => {}
        },
    }
    
    false
}

fn get_centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = if area.width > width {
        area.x + (area.width - width) / 2
    } else {
        area.x
    };
    let y = if area.height > height {
        area.y + (area.height - height) / 2
    } else {
        area.y
    };
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn draw_ui(f: &mut Frame, game: &SnakeGame) {
    let size = f.area();
    
    let target_width = 82;
    let target_height = 27;
    
    if size.width < target_width || size.height < target_height {
        let warning_text = vec![
            Line::from(Span::styled("⚠️  TERMINAL TOO SMALL", Style::default().fg(Color::Rgb(255, 60, 60)).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Please resize your terminal to at least ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{}x{}", target_width, target_height), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Current size: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{}x{}", size.width, size.height), Style::default().fg(Color::Rgb(255, 180, 0))),
            ]),
        ];
        
        let paragraph = Paragraph::new(warning_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(255, 60, 60))))
            .alignment(Alignment::Center);
            
        let centered_area = get_centered_rect(size, 45, 8);
        f.render_widget(paragraph, centered_area);
        return;
    }
    
    let centered_area = get_centered_rect(size, target_width, target_height);
    
    match game.state {
        GameState::StartScreen => {
            draw_start_screen(f, game, centered_area);
        }
        GameState::Playing | GameState::Paused | GameState::GameOver => {
            draw_game_screen(f, game, centered_area);
        }
    }
}

fn draw_start_screen(f: &mut Frame, game: &SnakeGame, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(9),  // Logo
            Constraint::Length(8),  // Difficulty selector
            Constraint::Length(10), // Controls / High Score
        ])
        .split(area);
        
    let logo_chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(chunks[0]);
        
    const ASCII_LOGO: &[&str] = &[
        "  ██████   ██████   ██   ██   ███    ██   █████   ██  ██   ███████ ",
        " ██       ██        ██   ██   ████   ██  ██   ██  ██ ██    ██      ",
        "  █████    █████    ███████   ██ ██  ██  ███████  ████     █████   ",
        "      ██       ██   ██   ██   ██  ██ ██  ██   ██  ██ ██    ██      ",
        " ██████   ██████    ██   ██   ██   ████  ██   ██  ██  ██   ███████ ",
    ];
    
    let colors = [
        Color::Rgb(0, 255, 127),   // Spring Green
        Color::Rgb(0, 255, 255),   // Cyan
        Color::Rgb(30, 144, 255),  // Dodger Blue
        Color::Rgb(138, 43, 226),  // Blue Violet
        Color::Rgb(255, 20, 147),  // Deep Pink
    ];
    
    let mut logo_lines = Vec::new();
    for (i, line) in ASCII_LOGO.iter().enumerate() {
        logo_lines.push(Line::from(Span::styled(
            *line,
            Style::default().fg(colors[i % colors.len()]).add_modifier(Modifier::BOLD)
        )));
    }
    let logo_para = Paragraph::new(logo_lines).alignment(Alignment::Center);
    f.render_widget(logo_para, logo_chunks[0]);
    
    let subtitle = Paragraph::new(Line::from(vec![
        Span::styled("A Vim-Friendly Snake Game in Rust", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
    ])).alignment(Alignment::Center);
    f.render_widget(subtitle, logo_chunks[2]);
    
    let difficulties = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard, Difficulty::Insane];
    let mut list_items = Vec::new();
    for (i, diff) in difficulties.iter().enumerate() {
        let is_selected = i == game.menu_selection;
        let text = match diff {
            Difficulty::Easy => "🟢  EASY    - Relaxed pace (150ms)",
            Difficulty::Medium => "🟡  MEDIUM  - Standard speed (100ms)",
            Difficulty::Hard => "🟠  HARD    - Test your limits (70ms)",
            Difficulty::Insane => "🔴  INSANE  - Absolute chaos (45ms)",
        };
        
        if is_selected {
            list_items.push(ListItem::new(Line::from(vec![
                Span::styled(" ▶ ", Style::default().fg(Color::Rgb(0, 255, 127)).add_modifier(Modifier::BOLD)),
                Span::styled(text, Style::default().fg(Color::Rgb(0, 255, 127)).add_modifier(Modifier::BOLD)),
                Span::styled(" ◀", Style::default().fg(Color::Rgb(0, 255, 127)).add_modifier(Modifier::BOLD)),
            ])));
        } else {
            list_items.push(ListItem::new(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(text, Style::default().fg(Color::Gray)),
            ])));
        }
    }
    
    let diff_list = List::new(list_items)
        .block(Block::default()
            .title(" SELECT DIFFICULTY ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(70, 130, 180))));
            
    f.render_widget(diff_list, chunks[1]);
    
    let info_text = vec![
        Line::from(vec![
            Span::styled("🏆  HIGH SCORE TO BEAT: ", Style::default().fg(Color::Rgb(255, 215, 0))),
            Span::styled(format!("{:03}", game.high_score), Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Controls: ", Style::default().fg(Color::Gray)),
            Span::styled("Arrow Keys / H J K L / W A S D", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(" to select", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::Gray)),
            Span::styled("Space / Enter", Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
            Span::styled(" to Start  │  Press ", Style::default().fg(Color::Gray)),
            Span::styled("Q / Esc", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
            Span::styled(" to Exit", Style::default().fg(Color::Gray)),
        ]),
    ];
    
    let info_para = Paragraph::new(info_text).alignment(Alignment::Center);
    f.render_widget(info_para, chunks[2]);
}

fn draw_game_screen(f: &mut Frame, game: &SnakeGame, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(22), // Playground
            Constraint::Length(2),  // Footer
        ])
        .split(area);
        
    let score_text = vec![
        Line::from(vec![
            Span::styled(" 🐍 SSHNAKE ", Style::default().fg(Color::Rgb(0, 255, 127)).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled("Score: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:03}", game.score), Style::default().fg(Color::Rgb(255, 180, 0)).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled("High Score: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:03}", game.high_score), Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("Level: {}", game.level), Style::default().fg(Color::Rgb(0, 255, 255))),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(game.difficulty.name(), Style::default().fg(Color::Rgb(255, 20, 147)).add_modifier(Modifier::BOLD)),
        ])
    ];
    
    let header = Paragraph::new(score_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)))
        .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);
    
    let board = GameBoard {
        snake: &game.snake,
        food: &game.food,
        current_dir: game.current_dir,
        state: game.state,
        tick_count: game.tick_count,
    };
    f.render_widget(board, chunks[1]);
    
    let footer_text = Line::from(vec![
        Span::styled(" Move: ", Style::default().fg(Color::Gray)),
        Span::styled("◀▲▼▶ / HJKL / WASD", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Pause: ", Style::default().fg(Color::Gray)),
        Span::styled("Space", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Restart: ", Style::default().fg(Color::Gray)),
        Span::styled("R", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Quit: ", Style::default().fg(Color::Gray)),
        Span::styled("Q / Esc", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]);
    let footer = Paragraph::new(footer_text).alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
    
    if game.state == GameState::Paused {
        let popup_area = get_centered_rect(chunks[1], 38, 8);
        f.render_widget(Clear, popup_area);
        
        let popup_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("   Press ", Style::default().fg(Color::Gray)),
                Span::styled("Space / P", Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                Span::styled(" to Resume", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("   Press ", Style::default().fg(Color::Gray)),
                Span::styled("R", Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD)),
                Span::styled(" to Restart", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("   Press ", Style::default().fg(Color::Gray)),
                Span::styled("M", Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
                Span::styled(" to return to Menu", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("   Press ", Style::default().fg(Color::Gray)),
                Span::styled("Q", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                Span::styled(" to Quit", Style::default().fg(Color::Gray)),
            ]),
        ];
        
        let popup_block = Block::default()
            .title(" ⏸ GAME PAUSED ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(255, 180, 0)));
            
        let popup_para = Paragraph::new(popup_text).block(popup_block);
        f.render_widget(popup_para, popup_area);
    } else if game.state == GameState::GameOver {
        let popup_area = get_centered_rect(chunks[1], 42, 10);
        f.render_widget(Clear, popup_area);
        
        let is_new_high = game.score == game.high_score && game.score > 0;
        
        let mut popup_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("☠  FATAL COLLISION  ☠", Style::default().fg(Color::Rgb(220, 50, 50)).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
        ];
        
        if is_new_high {
            popup_text.push(Line::from(vec![
                Span::styled("🎉 NEW HIGH SCORE: ", Style::default().fg(Color::Rgb(0, 255, 127)).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}", game.score), Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD)),
            ]));
        } else {
            popup_text.push(Line::from(vec![
                Span::styled("Final Score: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{}", game.score), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled("  │  High Score: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{}", game.high_score), Style::default().fg(Color::White)),
            ]));
        }
        
        popup_text.extend(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Gray)),
                Span::styled("R", Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                Span::styled(" to Try Again", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Gray)),
                Span::styled("M", Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
                Span::styled(" to return to Menu", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Gray)),
                Span::styled("Q", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                Span::styled(" to Quit", Style::default().fg(Color::Gray)),
            ]),
        ]);
        
        let popup_block = Block::default()
            .title(" ☠ GAME OVER ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(220, 50, 50)));
            
        let popup_para = Paragraph::new(popup_text).block(popup_block).alignment(Alignment::Center);
        f.render_widget(popup_para, popup_area);
    }
}

struct GameBoard<'a> {
    snake: &'a [Position],
    food: &'a Position,
    current_dir: Direction,
    state: GameState,
    tick_count: u64,
}

impl<'a> Widget for GameBoard<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = match self.state {
            GameState::GameOver => Color::Rgb(220, 50, 50),
            GameState::Paused => Color::Rgb(255, 180, 0),
            _ => Color::Rgb(70, 130, 180),
        };
        
        let block = Block::default()
            .title(" 🕹 SSHNAKE PLAYGROUND ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner_area = block.inner(area);
        block.render(area, buf);
        
        for y in 0..inner_area.height {
            for x in 0..inner_area.width {
                buf[(inner_area.x + x, inner_area.y + y)]
                    .set_char(' ')
                    .set_bg(Color::Rgb(18, 18, 18));
            }
        }
        
        let food_x = inner_area.x + (self.food.x * 2) as u16;
        let food_y = inner_area.y + self.food.y as u16;
        if food_x + 1 < inner_area.right() && food_y < inner_area.bottom() {
            let food_color = if self.tick_count % 2 == 0 {
                Color::Rgb(255, 60, 60)
            } else {
                Color::Rgb(255, 180, 0)
            };
            buf[(food_x, food_y)]
                .set_char('◆')
                .set_style(Style::default().fg(food_color).bg(Color::Rgb(18, 18, 18)).add_modifier(Modifier::BOLD));
            buf[(food_x + 1, food_y)]
                .set_char(' ')
                .set_style(Style::default().bg(Color::Rgb(18, 18, 18)));
        }
        
        if self.snake.is_empty() {
            return;
        }
        
        let n = self.snake.len();
        let head = self.snake[0];
        let head_x = inner_area.x + (head.x * 2) as u16;
        let head_y = inner_area.y + head.y as u16;
        
        let is_game_over = self.state == GameState::GameOver;
        
        if head_x + 1 < inner_area.right() && head_y < inner_area.bottom() {
            let (head_bg, head_fg, eye1, eye2) = if is_game_over {
                (Color::Rgb(120, 30, 30), Color::Rgb(255, 100, 100), 'x', 'x')
            } else {
                let (e1, e2) = match self.current_dir {
                    Direction::Up => ('^', '^'),
                    Direction::Down => ('v', 'v'),
                    Direction::Left => ('<', 'o'),
                    Direction::Right => ('o', '>'),
                };
                (Color::Rgb(0, 255, 127), Color::Rgb(0, 0, 0), e1, e2)
            };
            
            let head_style = Style::default().fg(head_fg).bg(head_bg).add_modifier(Modifier::BOLD);
            buf[(head_x, head_y)]
                .set_char(eye1)
                .set_style(head_style);
            buf[(head_x + 1, head_y)]
                .set_char(eye2)
                .set_style(head_style);
        }
        
        for (i, pos) in self.snake.iter().enumerate().skip(1) {
            let body_x = inner_area.x + (pos.x * 2) as u16;
            let body_y = inner_area.y + pos.y as u16;
            
            if body_x + 1 < inner_area.right() && body_y < inner_area.bottom() {
                let body_style = if is_game_over {
                    let ratio = i as f32 / n as f32;
                    let val = (100.0 * (1.0 - ratio) + 40.0 * ratio) as u8;
                    Style::default().bg(Color::Rgb(val, val, val))
                } else {
                    let ratio = i as f32 / n as f32;
                    let r = (0.0 * (1.0 - ratio) + 10.0 * ratio) as u8;
                    let g = (230.0 * (1.0 - ratio) + 60.0 * ratio) as u8;
                    let b = (110.0 * (1.0 - ratio) + 25.0 * ratio) as u8;
                    Style::default().bg(Color::Rgb(r, g, b))
                };
                
                buf[(body_x, body_y)]
                    .set_char(' ')
                    .set_style(body_style);
                buf[(body_x + 1, body_y)]
                    .set_char(' ')
                    .set_style(body_style);
            }
        }
    }
}
