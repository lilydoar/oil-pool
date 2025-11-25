//! Geometry generators for tic-tac-toe rendering
//!
//! Provides functions to generate line segments for board, pieces, and numbers

use super::line_renderer::Line;

/// Parameters for tic-tac-toe board layout
pub struct BoardLayout {
    pub center_x: f32,
    pub center_y: f32,
    pub cell_size: f32,
    pub line_thickness: f32,
}

impl BoardLayout {
    /// Creates a centered board layout for the given screen size
    pub fn centered(screen_width: f32, screen_height: f32) -> Self {
        let size = screen_width.min(screen_height) * 0.6;
        Self {
            center_x: screen_width / 2.0,
            center_y: screen_height / 2.0,
            cell_size: size / 3.0,
            line_thickness: 4.0,
        }
    }

    /// Gets the screen position for a board cell (row, col both 0-2)
    pub fn cell_center(&self, row: usize, col: usize) -> [f32; 2] {
        let board_width = self.cell_size * 3.0;
        let top_left_x = self.center_x - board_width / 2.0;
        let top_left_y = self.center_y - board_width / 2.0;

        [
            top_left_x + (col as f32 + 0.5) * self.cell_size,
            top_left_y + (row as f32 + 0.5) * self.cell_size,
        ]
    }

    /// Converts screen coordinates to board cell (row, col)
    /// Returns None if outside the board
    pub fn screen_to_cell(&self, screen_x: f32, screen_y: f32) -> Option<(usize, usize)> {
        let board_width = self.cell_size * 3.0;
        let top_left_x = self.center_x - board_width / 2.0;
        let top_left_y = self.center_y - board_width / 2.0;

        let rel_x = screen_x - top_left_x;
        let rel_y = screen_y - top_left_y;

        if rel_x < 0.0 || rel_y < 0.0 || rel_x >= board_width || rel_y >= board_width {
            return None;
        }

        let col = (rel_x / self.cell_size) as usize;
        let row = (rel_y / self.cell_size) as usize;

        if row < 3 && col < 3 {
            Some((row, col))
        } else {
            None
        }
    }
}

/// Generates lines for the tic-tac-toe board grid
pub fn generate_board_grid(layout: &BoardLayout) -> Vec<Line> {
    let mut lines = Vec::new();
    let board_width = layout.cell_size * 3.0;
    let left = layout.center_x - board_width / 2.0;
    let right = layout.center_x + board_width / 2.0;
    let top = layout.center_y - board_width / 2.0;
    let bottom = layout.center_y + board_width / 2.0;

    // Horizontal lines (2)
    for i in 1..3 {
        let y = top + i as f32 * layout.cell_size;
        lines.push(Line::new([left, y], [right, y], layout.line_thickness));
    }

    // Vertical lines (2)
    for i in 1..3 {
        let x = left + i as f32 * layout.cell_size;
        lines.push(Line::new([x, top], [x, bottom], layout.line_thickness));
    }

    lines
}

/// Generates lines for an X symbol at the given cell
pub fn generate_x(layout: &BoardLayout, row: usize, col: usize) -> Vec<Line> {
    let center = layout.cell_center(row, col);
    let size = layout.cell_size * 0.6;
    let half = size / 2.0;

    vec![
        // Diagonal from top-left to bottom-right
        Line::new(
            [center[0] - half, center[1] - half],
            [center[0] + half, center[1] + half],
            layout.line_thickness,
        ),
        // Diagonal from top-right to bottom-left
        Line::new(
            [center[0] + half, center[1] - half],
            [center[0] - half, center[1] + half],
            layout.line_thickness,
        ),
    ]
}

/// Generates lines for an O symbol at the given cell
pub fn generate_o(layout: &BoardLayout, row: usize, col: usize) -> Vec<Line> {
    let center = layout.cell_center(row, col);
    let radius = layout.cell_size * 0.3;
    let segments = 32; // Number of line segments for circle

    let mut lines = Vec::new();
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
        let angle2 = ((i + 1) as f32 / segments as f32) * 2.0 * std::f32::consts::PI;

        let x1 = center[0] + radius * angle1.cos();
        let y1 = center[1] + radius * angle1.sin();
        let x2 = center[0] + radius * angle2.cos();
        let y2 = center[1] + radius * angle2.sin();

        lines.push(Line::new([x1, y1], [x2, y2], layout.line_thickness));
    }

    lines
}

/// 7-segment display patterns for digits 0-9
/// Each segment is represented as (start_x, start_y, end_x, end_y) in normalized coordinates
const DIGIT_SEGMENTS: [&[(f32, f32, f32, f32)]; 10] = [
    // 0
    &[
        (0.0, 0.0, 1.0, 0.0), // top
        (0.0, 0.0, 0.0, 0.5), // top-left
        (1.0, 0.0, 1.0, 0.5), // top-right
        (0.0, 0.5, 0.0, 1.0), // bottom-left
        (1.0, 0.5, 1.0, 1.0), // bottom-right
        (0.0, 1.0, 1.0, 1.0), // bottom
    ],
    // 1
    &[
        (1.0, 0.0, 1.0, 0.5), // top-right
        (1.0, 0.5, 1.0, 1.0), // bottom-right
    ],
    // 2
    &[
        (0.0, 0.0, 1.0, 0.0), // top
        (1.0, 0.0, 1.0, 0.5), // top-right
        (0.0, 0.5, 1.0, 0.5), // middle
        (0.0, 0.5, 0.0, 1.0), // bottom-left
        (0.0, 1.0, 1.0, 1.0), // bottom
    ],
    // 3
    &[
        (0.0, 0.0, 1.0, 0.0), // top
        (1.0, 0.0, 1.0, 0.5), // top-right
        (0.0, 0.5, 1.0, 0.5), // middle
        (1.0, 0.5, 1.0, 1.0), // bottom-right
        (0.0, 1.0, 1.0, 1.0), // bottom
    ],
    // 4
    &[
        (0.0, 0.0, 0.0, 0.5), // top-left
        (0.0, 0.5, 1.0, 0.5), // middle
        (1.0, 0.0, 1.0, 0.5), // top-right
        (1.0, 0.5, 1.0, 1.0), // bottom-right
    ],
    // 5
    &[
        (0.0, 0.0, 1.0, 0.0), // top
        (0.0, 0.0, 0.0, 0.5), // top-left
        (0.0, 0.5, 1.0, 0.5), // middle
        (1.0, 0.5, 1.0, 1.0), // bottom-right
        (0.0, 1.0, 1.0, 1.0), // bottom
    ],
    // 6
    &[
        (0.0, 0.0, 1.0, 0.0), // top
        (0.0, 0.0, 0.0, 0.5), // top-left
        (0.0, 0.5, 1.0, 0.5), // middle
        (0.0, 0.5, 0.0, 1.0), // bottom-left
        (1.0, 0.5, 1.0, 1.0), // bottom-right
        (0.0, 1.0, 1.0, 1.0), // bottom
    ],
    // 7
    &[
        (0.0, 0.0, 1.0, 0.0), // top
        (1.0, 0.0, 1.0, 0.5), // top-right
        (1.0, 0.5, 1.0, 1.0), // bottom-right
    ],
    // 8
    &[
        (0.0, 0.0, 1.0, 0.0), // top
        (0.0, 0.0, 0.0, 0.5), // top-left
        (1.0, 0.0, 1.0, 0.5), // top-right
        (0.0, 0.5, 1.0, 0.5), // middle
        (0.0, 0.5, 0.0, 1.0), // bottom-left
        (1.0, 0.5, 1.0, 1.0), // bottom-right
        (0.0, 1.0, 1.0, 1.0), // bottom
    ],
    // 9
    &[
        (0.0, 0.0, 1.0, 0.0), // top
        (0.0, 0.0, 0.0, 0.5), // top-left
        (1.0, 0.0, 1.0, 0.5), // top-right
        (0.0, 0.5, 1.0, 0.5), // middle
        (1.0, 0.5, 1.0, 1.0), // bottom-right
        (0.0, 1.0, 1.0, 1.0), // bottom
    ],
];

/// Generates lines for a digit (0-9) at the given position
pub fn generate_digit(
    digit: u32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    thickness: f32,
) -> Vec<Line> {
    if digit > 9 {
        return vec![];
    }

    let segments = DIGIT_SEGMENTS[digit as usize];
    let mut lines = Vec::new();

    for &(x1, y1, x2, y2) in segments {
        let screen_x1 = x + x1 * width;
        let screen_y1 = y + y1 * height;
        let screen_x2 = x + x2 * width;
        let screen_y2 = y + y2 * height;

        lines.push(Line::new(
            [screen_x1, screen_y1],
            [screen_x2, screen_y2],
            thickness,
        ));
    }

    lines
}

/// Generates lines for a multi-digit number at the given position
pub fn generate_number(
    number: u32,
    x: f32,
    y: f32,
    digit_width: f32,
    digit_height: f32,
    spacing: f32,
    thickness: f32,
) -> Vec<Line> {
    let mut lines = Vec::new();
    let digits: Vec<u32> = number
        .to_string()
        .chars()
        .map(|c| c.to_digit(10).unwrap())
        .collect();

    for (i, &digit) in digits.iter().enumerate() {
        let digit_x = x + i as f32 * (digit_width + spacing);
        lines.extend(generate_digit(
            digit,
            digit_x,
            y,
            digit_width,
            digit_height,
            thickness,
        ));
    }

    lines
}
