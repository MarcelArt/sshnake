use std::fs;
use rand::RngExt;

pub const GRID_WIDTH: i32 = 40;
pub const GRID_HEIGHT: i32 = 20;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameState {
    StartScreen,
    Playing,
    Paused,
    GameOver,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Insane,
}

impl Difficulty {
    pub fn base_tick_rate(&self) -> std::time::Duration {
        match self {
            Difficulty::Easy => std::time::Duration::from_millis(150),
            Difficulty::Medium => std::time::Duration::from_millis(100),
            Difficulty::Hard => std::time::Duration::from_millis(70),
            Difficulty::Insane => std::time::Duration::from_millis(45),
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
            Difficulty::Insane => "Insane",
        }
    }
}

pub struct SnakeGame {
    pub snake: Vec<Position>,
    pub food: Position,
    pub current_dir: Direction,
    pub next_dir: Direction,
    pub score: u32,
    pub high_score: u32,
    pub state: GameState,
    pub difficulty: Difficulty,
    pub tick_count: u64,
    pub level: u32,
    pub menu_selection: usize,
    high_score_file: String,
}

impl SnakeGame {
    pub fn new() -> Self {
        let high_score_file = ".sshnake_highscore".to_string();
        let high_score = fs::read_to_string(&high_score_file)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        
        let mut game = Self {
            snake: Vec::new(),
            food: Position { x: 0, y: 0 },
            current_dir: Direction::Right,
            next_dir: Direction::Right,
            score: 0,
            high_score,
            state: GameState::StartScreen,
            difficulty: Difficulty::Medium,
            tick_count: 0,
            level: 1,
            menu_selection: 1, // Medium starts selected
            high_score_file,
        };
        game.reset();
        game
    }
    
    pub fn reset(&mut self) {
        let start_x = GRID_WIDTH / 2;
        let start_y = GRID_HEIGHT / 2;
        self.snake = vec![
            Position { x: start_x, y: start_y },
            Position { x: start_x - 1, y: start_y },
            Position { x: start_x - 2, y: start_y },
        ];
        self.current_dir = Direction::Right;
        self.next_dir = Direction::Right;
        self.score = 0;
        self.level = 1;
        self.tick_count = 0;
        self.spawn_food();
    }
    
    pub fn spawn_food(&mut self) {
        let mut rng = rand::rng();
        loop {
            let x = rng.random_range(0..GRID_WIDTH);
            let y = rng.random_range(0..GRID_HEIGHT);
            let pos = Position { x, y };
            
            if !self.snake.contains(&pos) {
                self.food = pos;
                break;
            }
        }
    }
    
    pub fn tick_rate(&self) -> std::time::Duration {
        // Decrease tick duration slightly as level increases
        let base = self.difficulty.base_tick_rate();
        let speed_up_ms = (self.level - 1) as u64 * 3;
        let min_duration = std::time::Duration::from_millis(30);
        
        base.checked_sub(std::time::Duration::from_millis(speed_up_ms))
            .unwrap_or(min_duration)
            .max(min_duration)
    }
    
    pub fn tick(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        
        self.tick_count += 1;
        self.current_dir = self.next_dir;
        
        let head = self.snake[0];
        let new_head = match self.current_dir {
            Direction::Up => Position { x: head.x, y: head.y - 1 },
            Direction::Down => Position { x: head.x, y: head.y + 1 },
            Direction::Left => Position { x: head.x - 1, y: head.y },
            Direction::Right => Position { x: head.x + 1, y: head.y },
        };
        
        // Check wall collision
        if new_head.x < 0 || new_head.x >= GRID_WIDTH || new_head.y < 0 || new_head.y >= GRID_HEIGHT {
            self.state = GameState::GameOver;
            self.check_high_score();
            return;
        }
        
        // Check self collision (collision with any body segment except the tail when we're moving, 
        // but checking the whole snake is safer and standard since head cannot hit tail directly unless length is small)
        if self.snake.contains(&new_head) {
            self.state = GameState::GameOver;
            self.check_high_score();
            return;
        }
        
        self.snake.insert(0, new_head);
        
        if new_head == self.food {
            self.score += 10;
            self.level = (self.score / 50) + 1;
            self.spawn_food();
        } else {
            self.snake.pop();
        }
    }
    
    pub fn check_high_score(&mut self) {
        if self.score > self.high_score {
            self.high_score = self.score;
            let _ = fs::write(&self.high_score_file, self.high_score.to_string());
        }
    }
}
