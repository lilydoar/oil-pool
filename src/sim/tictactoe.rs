use enum_map::{Enum, EnumMap};
use serde::{Deserialize, Serialize};

use super::Simulation;

/// TicTacToe simulation - a pure game logic implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicTacToeSimulation {
    /// 3x3 board state, indexed as [row][col]
    board: [[Tile; 3]; 3],
    /// Current player
    current_player: Player,
    /// Game state
    game_state: GameState,
    /// Score tracking across multiple games
    score: Score,
}

/// Player markers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Enum)]
pub enum Player {
    X,
    O,
}

/// Tile state on the board
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Tile {
    #[default]
    Empty,
    X,
    O,
}

/// Current game state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameState {
    Playing,
    Won(Player),
    Draw,
}

/// Score tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Score {
    wins: EnumMap<Player, u32>,
    draws: u32,
}

impl Player {
    /// Returns the opposite player
    pub fn opponent(self) -> Self {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }

    /// Convert player to tile
    pub fn to_tile(self) -> Tile {
        match self {
            Player::X => Tile::X,
            Player::O => Tile::O,
        }
    }
}

impl TicTacToeSimulation {
    /// Creates a new TicTacToe game
    pub fn new() -> Self {
        Self {
            board: [[Tile::Empty; 3]; 3],
            current_player: Player::X,
            game_state: GameState::Playing,
            score: Score::default(),
        }
    }

    /// Attempts to make a move at the given board position (0-2, 0-2)
    /// Returns true if the move was successful
    pub fn make_move(&mut self, row: usize, col: usize) -> bool {
        // Validate move
        if !matches!(self.game_state, GameState::Playing) {
            return false;
        }
        if row >= 3 || col >= 3 {
            return false;
        }
        if self.board[row][col] != Tile::Empty {
            return false;
        }

        // Place the piece
        self.board[row][col] = self.current_player.to_tile();

        // Check for win or draw
        if self.check_win() {
            self.game_state = GameState::Won(self.current_player);
            self.score.wins[self.current_player] += 1;
        } else if self.is_board_full() {
            self.game_state = GameState::Draw;
            self.score.draws += 1;
        } else {
            // Switch player
            self.current_player = self.current_player.opponent();
        }

        true
    }

    /// Resets the board for a new game, keeping scores
    pub fn reset(&mut self) {
        self.board = [[Tile::Empty; 3]; 3];
        self.current_player = Player::X;
        self.game_state = GameState::Playing;
    }

    /// Checks if the current player has won
    fn check_win(&self) -> bool {
        let tile = self.current_player.to_tile();

        // Check rows
        for row in 0..3 {
            if self.board[row][0] == tile
                && self.board[row][1] == tile
                && self.board[row][2] == tile
            {
                return true;
            }
        }

        // Check columns
        for col in 0..3 {
            if self.board[0][col] == tile
                && self.board[1][col] == tile
                && self.board[2][col] == tile
            {
                return true;
            }
        }

        // Check diagonals
        if self.board[0][0] == tile && self.board[1][1] == tile && self.board[2][2] == tile {
            return true;
        }
        if self.board[0][2] == tile && self.board[1][1] == tile && self.board[2][0] == tile {
            return true;
        }

        false
    }

    /// Checks if the board is completely full
    fn is_board_full(&self) -> bool {
        self.board
            .iter()
            .all(|row| row.iter().all(|&tile| tile != Tile::Empty))
    }

    // Public accessors for rendering

    /// Returns the current board state
    pub fn board(&self) -> &[[Tile; 3]; 3] {
        &self.board
    }

    /// Returns the current player
    pub fn current_player(&self) -> Player {
        self.current_player
    }

    /// Returns the game state
    pub fn game_state(&self) -> GameState {
        self.game_state
    }

    /// Returns the current scores
    pub fn score(&self) -> &Score {
        &self.score
    }

    /// Returns wins for a specific player
    pub fn wins(&self, player: Player) -> u32 {
        self.score.wins[player]
    }

    /// Returns total draws
    pub fn draws(&self) -> u32 {
        self.score.draws
    }
}

impl Default for TicTacToeSimulation {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulation for TicTacToeSimulation {
    fn tick(&mut self, _delta_time: f32) {
        // TicTacToe is turn-based, no continuous simulation needed
    }

    fn reset(&mut self) {
        self.reset();
    }

    fn name(&self) -> &str {
        "tictactoe"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
