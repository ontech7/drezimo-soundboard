//! The display's icons: the window's own (src/icons.tsx), redrawn pixel by pixel, most of them 9 pixels high.

pub struct Icon {
    rows: &'static [&'static str],
}

impl Icon {
    pub fn width(&self) -> i32 {
        self.rows.first().map_or(0, |row| row.len() as i32)
    }

    /// The lit pixels, from the top left.
    pub fn pixels(&self) -> impl Iterator<Item = (i32, i32)> {
        self.rows.iter().enumerate().flat_map(|(y, row)| {
            row.bytes()
                .enumerate()
                .filter(|(_, pixel)| *pixel == b'#')
                .map(move |(x, _)| (x as i32, y as i32))
        })
    }
}

/// The voice loaded
pub const WAVE: Icon = Icon {
    rows: &[
        "....#....",
        "..#.#....",
        "..#.#.#..",
        "#.#.#.#.#",
        "#.#.#.#.#",
        "#.#.#.#.#",
        "..#.#.#..",
        "..#.#....",
        "....#....",
    ],
};

/// The voice changer off: the wave flattened
pub const WAVE_OFF: Icon = Icon {
    rows: &[
        ".........",
        ".........",
        ".........",
        ".........",
        "#.#.#.#.#",
        ".........",
        ".........",
        ".........",
        ".........",
    ],
};

pub const MIC_OFF: Icon = Icon {
    rows: &[
        "..###......",
        "..###......",
        "..###......",
        "#.###.#.#.#",
        "#.###.#..#.",
        ".#...#..#.#",
        "..###......",
        "...#.......",
        "...#.......",
    ],
};

/// The sounds muted for this computer only
pub const SPEAKER_OFF: Icon = Icon {
    rows: &[
        ".........",
        "...#.....",
        "..##.....",
        "####..#.#",
        "####...#.",
        "####..#.#",
        "..##.....",
        "...#.....",
        ".........",
    ],
};

/// Hearing one's own voice
pub const HEADPHONES: Icon = Icon {
    rows: &[
        "..#####..",
        ".#.....#.",
        "#.......#",
        "#.......#",
        "##.....##",
        "##.....##",
        "##.....##",
        "##.....##",
        ".........",
    ],
};

pub const PLAY: Icon = Icon {
    rows: &["#...", "##..", "###.", "####", "###.", "##..", "#..."],
};

pub const PAUSE: Icon = Icon {
    rows: &["##.##", "##.##", "##.##", "##.##", "##.##", "##.##", "##.##"],
};

pub const LOOP: Icon = Icon {
    rows: &[
        ".....#...",
        ".######..",
        "#....#...",
        "#.......#",
        "...#....#",
        "..######.",
        "...#.....",
    ],
};

pub const MACRO: Icon = Icon {
    rows: &[
        "#####....",
        "......#..",
        "......##.",
        "###...###",
        "......##.",
        "......#..",
        "#####....",
    ],
};

/// A sound that stops the others
pub const STOP: Icon = Icon {
    rows: &["#####", "#####", "#####", "#####", "#####"],
};

/// Outside Voicemod's licence
pub const LOCK: Icon = Icon {
    rows: &[
        "..###..",
        ".#...#.",
        ".#...#.",
        "#######",
        "###.###",
        "###.###",
        "#######",
    ],
};

/// The computer, where Voicemod runs
pub const MONITOR: Icon = Icon {
    rows: &[
        "###############",
        "#.............#",
        "#.............#",
        "#.............#",
        "#.............#",
        "#.............#",
        "#.............#",
        "#.............#",
        "###############",
        "......###......",
        "....#######....",
    ],
};
